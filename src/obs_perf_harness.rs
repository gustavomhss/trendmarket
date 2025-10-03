use std::collections::BTreeMap;
use std::hint::black_box;
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::amm::pricing::{
    execution_price_x_to_y, max_in_with_tolerance, min_out_with_tolerance, slippage_ppm_x_to_y,
};
use crate::amm::swap::get_amount_out;
use crate::amm::types::{Ppm, Wad, PPM_SCALE, U256, WAD};
use crate::amm::{guardrails::u256_to_u128_checked, types::MIN_RESERVE};
use crate::telemetry_spans_amm::{in_amm_swap, in_pricing_quote, PricingQuoteAttrs, SwapAttrs};
use crate::telemetry_spans_cdc::{in_cdc_consume, CdcConsumeAttrs};
use anyhow::Result;
use opentelemetry::global;
use opentelemetry::KeyValue;
use opentelemetry_sdk::metrics::SdkMeterProvider;
use tracing::level_filters::LevelFilter;
use tracing::subscriber::{DefaultGuard, NoSubscriber};

const SERVICE_NAME: &str = "ce-amm";
const SERVICE_ENV: &str = "dev";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsPerfMode {
    Off,
    Min,
    Full,
}

impl ObsPerfMode {
    pub fn all() -> [Self; 3] {
        [Self::Off, Self::Min, Self::Full]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Min => "min",
            Self::Full => "full",
        }
    }
}

impl FromStr for ObsPerfMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "min" => Ok(Self::Min),
            "full" => Ok(Self::Full),
            other => Err(format!("invalid mode `{other}`; expected off|min|full")),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct HarnessConfig {
    pub otlp_endpoint: Option<String>,
    pub enable_prometheus: bool,
}

#[derive(Debug, Clone)]
pub struct WorkloadMetrics {
    pub ops: u64,
    pub duration: Duration,
}

impl WorkloadMetrics {
    pub fn ns_per_op(&self) -> f64 {
        if self.ops == 0 {
            return 0.0;
        }
        self.duration.as_secs_f64() * 1_000_000_000.0 / (self.ops as f64)
    }

    pub fn duration_secs(&self) -> f64 {
        self.duration.as_secs_f64()
    }
}

#[derive(Debug, Clone)]
pub struct ModeRunSummary {
    pub workloads: BTreeMap<&'static str, WorkloadMetrics>,
    pub total_duration: Duration,
}

struct ObservabilityGuards {
    meter_provider: Option<SdkMeterProvider>,
    subscriber_guard: Option<DefaultGuard>,
}

impl ObservabilityGuards {
    fn new(meter_provider: Option<SdkMeterProvider>, subscriber_guard: DefaultGuard) -> Self {
        Self {
            meter_provider,
            subscriber_guard: Some(subscriber_guard),
        }
    }
}

impl Drop for ObservabilityGuards {
    fn drop(&mut self) {
        if let Some(guard) = self.subscriber_guard.take() {
            drop(guard);
        }
        if let Some(provider) = self.meter_provider.take() {
            let _ = provider.shutdown();
        }
    }
}

pub fn run_mode(
    mode: ObsPerfMode,
    ops_per_workload: u64,
    config: &HarnessConfig,
) -> Result<ModeRunSummary> {
    let (_guards, workloads, total_duration) = with_observability(mode, config, || {
        let mut results = BTreeMap::new();
        results.insert("amm", run_amm_swap_cycle(ops_per_workload));
        results.insert("pricing", run_pricing_quote_cycle(ops_per_workload));
        results.insert("cdc", run_cdc_consume_cycle(ops_per_workload));
        results
    })?;

    Ok(ModeRunSummary {
        workloads,
        total_duration,
    })
}

pub fn run_single_workload(
    mode: ObsPerfMode,
    workload: &'static str,
    ops: u64,
    config: &HarnessConfig,
) -> Result<WorkloadMetrics> {
    let (_guards, mut map, _) = with_observability(mode, config, || match workload {
        "amm" => {
            let mut m = BTreeMap::new();
            m.insert("amm", run_amm_swap_cycle(ops));
            m
        }
        "pricing" => {
            let mut m = BTreeMap::new();
            m.insert("pricing", run_pricing_quote_cycle(ops));
            m
        }
        "cdc" => {
            let mut m = BTreeMap::new();
            m.insert("cdc", run_cdc_consume_cycle(ops));
            m
        }
        other => panic!("unknown workload `{other}`"),
    })?;

    Ok(map.remove(workload).unwrap_or(WorkloadMetrics {
        ops: 0,
        duration: Duration::default(),
    }))
}

fn with_observability<F>(
    mode: ObsPerfMode,
    config: &HarnessConfig,
    f: F,
) -> Result<(
    ObservabilityGuards,
    BTreeMap<&'static str, WorkloadMetrics>,
    Duration,
)>
where
    F: FnOnce() -> BTreeMap<&'static str, WorkloadMetrics>,
{
    let guards = install_observability(mode, config)?;
    let start = Instant::now();
    let summary = f();
    let elapsed = start.elapsed();
    Ok((guards, summary, elapsed))
}

fn install_observability(mode: ObsPerfMode, config: &HarnessConfig) -> Result<ObservabilityGuards> {
    let service_version = format!("{}-perf", env!("CARGO_PKG_VERSION"));

    let meter_resource = opentelemetry_sdk::Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", SERVICE_NAME),
            KeyValue::new("service.version", service_version.clone()),
            KeyValue::new("deployment.environment", SERVICE_ENV),
        ])
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(meter_resource)
        .build();
    let _ = global::set_meter_provider(meter_provider.clone());

    match mode {
        ObsPerfMode::Off => {
            let guard = tracing::subscriber::set_default(NoSubscriber::default());
            Ok(ObservabilityGuards::new(Some(meter_provider), guard))
        }
        ObsPerfMode::Min | ObsPerfMode::Full => {
            let max_level = match mode {
                ObsPerfMode::Min => LevelFilter::INFO,
                ObsPerfMode::Full => LevelFilter::DEBUG,
                ObsPerfMode::Off => unreachable!(),
            };

            let mut builder = tracing_subscriber::fmt()
                .with_writer(|| std::io::stderr())
                .with_target(false)
                .with_ansi(false)
                .with_level(true)
                .with_max_level(max_level);

            if mode == ObsPerfMode::Full {
                builder = builder.with_thread_ids(true).with_thread_names(true);
            }

            let subscriber = builder.finish();
            let guard = tracing::subscriber::set_default(subscriber);

            if config.enable_prometheus {
                tracing::debug!("prometheus exporter flag ignored in perf harness");
            }

            if config.otlp_endpoint.is_some() {
                tracing::debug!("OTLP endpoint flag ignored in perf harness");
            }

            Ok(ObservabilityGuards::new(Some(meter_provider), guard))
        }
    }
}

fn run_amm_swap_cycle(ops: u64) -> WorkloadMetrics {
    if ops == 0 {
        return WorkloadMetrics {
            ops,
            duration: Duration::ZERO,
        };
    }

    let mut x: Wad = 5_000_000 * WAD;
    let mut y: Wad = 4_500_000 * WAD;
    let fee_ppm: Ppm = 300;
    let mut accumulator: u128 = 0;

    let start = Instant::now();
    for i in 0..ops {
        if i % 7_500 == 0 && i != 0 {
            x = 5_000_000 * WAD;
            y = 4_500_000 * WAD;
        }

        let dx = compute_dx(i, x);
        let fee = fee_on_input(dx, fee_ppm);
        let out = get_amount_out(x, y, dx, fee_ppm).expect("swap must succeed");
        let x_after = x + dx - fee;
        let y_after = y - out;
        if y_after <= MIN_RESERVE {
            x = 5_000_000 * WAD;
            y = 4_500_000 * WAD;
            continue;
        }

        let k_before = invariant_as_f64(x, y);
        let k_after = invariant_as_f64(x_after, y_after);
        let attrs = SwapAttrs {
            k_before,
            k_after,
            delta_k_ratio: delta_ratio(k_before, k_after),
            fee_ppm: fee_ppm as i64,
            input: wad_to_f64(dx - fee),
            output: wad_to_f64(out),
        };

        in_amm_swap(&attrs, || {
            accumulator = accumulator.wrapping_add(out);
        });

        x = x_after;
        y = y_after;
    }

    black_box(accumulator);
    WorkloadMetrics {
        ops,
        duration: start.elapsed(),
    }
}

fn run_pricing_quote_cycle(ops: u64) -> WorkloadMetrics {
    if ops == 0 {
        return WorkloadMetrics {
            ops,
            duration: Duration::ZERO,
        };
    }

    let x: Wad = 6_000_000 * WAD;
    let y: Wad = 6_500_000 * WAD;
    let fee_ppm: Ppm = 500;
    let tolerance: Ppm = 10_000;
    let mut checksum: u128 = 0;

    let start = Instant::now();
    for i in 0..ops {
        let dx = compute_quote_dx(i);
        let exec_price = execution_price_x_to_y(x, y, dx, fee_ppm).expect("execution price");
        let min_out = min_out_with_tolerance(x, y, dx, fee_ppm, tolerance).expect("min out");
        let max_in = max_in_with_tolerance(x, y, min_out, fee_ppm, tolerance).expect("max in");
        let slippage = slippage_ppm_x_to_y(x, y, dx, fee_ppm).expect("slippage");

        let attrs = PricingQuoteAttrs {
            k_before: invariant_as_f64(x, y),
            k_after: invariant_as_f64(x, y),
            delta_k_ratio: 0.0,
            fee_ppm: fee_ppm as i64,
            input: wad_to_f64(dx),
            output: wad_to_f64(min_out),
        };

        in_pricing_quote(&attrs, || {
            let combo = exec_price as u128 ^ (max_in as u128);
            checksum = checksum.wrapping_add(combo + slippage as u128);
        });
    }

    black_box(checksum);
    WorkloadMetrics {
        ops,
        duration: start.elapsed(),
    }
}

fn run_cdc_consume_cycle(ops: u64) -> WorkloadMetrics {
    if ops == 0 {
        return WorkloadMetrics {
            ops,
            duration: Duration::ZERO,
        };
    }

    let stream = "ce.cdc.trades".to_string();
    let partitions = ["p0", "p1", "p2", "p3"];
    let mut offset: i64 = 0;
    let mut lag = 0.25f64;
    let mut checksum: i64 = 0;

    let start = Instant::now();
    for i in 0..ops {
        let records = (i % 50 + 1) as i64;
        let partition = partitions[(i as usize) % partitions.len()].to_string();
        let attrs = CdcConsumeAttrs {
            stream: stream.clone(),
            partition,
            offset_before: offset,
            offset_after: offset + records,
            records,
            lag_seconds: lag,
        };

        in_cdc_consume(&attrs, || {
            let mut local = 0i64;
            for r in 0..records {
                let val = offset + r;
                local ^= (val << (r % 3)) ^ ((val % 7) * 13);
            }
            checksum ^= local;
        });

        offset += records;
        lag = ((lag * 1.03).rem_euclid(5.0)).max(0.000_5);
    }

    black_box(checksum);
    WorkloadMetrics {
        ops,
        duration: start.elapsed(),
    }
}

fn compute_dx(iteration: u64, x: Wad) -> Wad {
    let base = (iteration % 1_000) as u128 + 1;
    let scaled = base * (WAD / 10_000); // 0.0001 .. 0.1000
    let cap = x / 50;
    scaled.min(cap.max(1))
}

fn compute_quote_dx(iteration: u64) -> Wad {
    let base = (iteration % 750) as u128 + 500;
    (base * (WAD / 5_000)).max(1)
}

fn fee_on_input(dx: Wad, fee_ppm: Ppm) -> Wad {
    if fee_ppm == 0 {
        return 0;
    }
    let numerator = U256::from(dx) * U256::from(fee_ppm as u64);
    let denom = U256::from(PPM_SCALE as u64);
    let adjustment = denom - U256::from(1u8);
    let fee = (numerator + adjustment) / denom;
    u256_to_u128_checked(fee).expect("fee within bounds")
}

fn wad_to_f64(value: Wad) -> f64 {
    (value as f64) / (WAD as f64)
}

fn invariant_as_f64(x: Wad, y: Wad) -> f64 {
    wad_to_f64(x) * wad_to_f64(y)
}

fn delta_ratio(before: f64, after: f64) -> f64 {
    if before.abs() < f64::EPSILON {
        0.0
    } else {
        (after - before) / before
    }
}
