use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use credit_engine_core::telemetry_cfg::{DeployEnv as CfgDeployEnv, ObsLevel, TelemetryConfig};
use credit_engine_core::telemetry_contract as contract;
use credit_engine_core::telemetry_identity::{
    DeployEnv as IdentityEnv, ServiceIdentity, ServiceIdentityBuilder,
};
use opentelemetry::metrics::{Counter, Histogram, Meter, MeterProvider};
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    propagation::TraceContextPropagator,
    resource::Resource,
    trace::SdkTracerProvider,
};
use serde_json::{Map as JsonMap, Value as JsonValue};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::{info, warn, Event, Span, Subscriber};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_subscriber::fmt::{self, format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

const DEFAULT_OPERATION_COUNT: usize = 20;
const SWAP_OP: &str = "swap";
const PRICING_OP: &str = "pricing";
const CDC_OP: &str = "cdc_consume";

#[tokio::main]
async fn main() -> Result<()> {
    let cfg =
        TelemetryConfig::from_env().context("failed to load TelemetryConfig from environment")?;
    let identity = ServiceIdentityBuilder::new()
        .with_service_name(cfg.service_name.clone())
        .with_service_version(cfg.service_version.clone())
        .with_deploy_env(convert_env(cfg.deploy_env.clone()))
        .build()
        .context("failed to construct ServiceIdentity")?;

    let runtime = Arc::new(TelemetryRuntime::initialize(cfg.clone(), identity.clone()).await?);
    let total_ops = parse_operation_target();

    println!(
        "obs_demo starting: service={} version={} env={} operations={}",
        identity.service_name, identity.service_version, identity.deploy_env, total_ops
    );

    let mut workload = SyntheticWorkload::new(runtime.clone());
    for step in 0..total_ops {
        workload.execute(step).await?;
    }

    runtime.shutdown().await;

    let summary = workload.summary();
    println!(
        "obs_demo completed: swap={} pricing={} cdc={}",
        summary.swap, summary.pricing, summary.cdc
    );

    Ok(())
}

fn parse_operation_target() -> usize {
    match env::var("OBS_DEMO_OPS") {
        Ok(raw) => raw
            .trim()
            .parse::<usize>()
            .ok()
            .filter(|value| *value > 0)
            .unwrap_or(DEFAULT_OPERATION_COUNT),
        Err(_) => DEFAULT_OPERATION_COUNT,
    }
}

fn convert_env(env: CfgDeployEnv) -> IdentityEnv {
    match env {
        CfgDeployEnv::Dev => IdentityEnv::Dev,
        CfgDeployEnv::Stg => IdentityEnv::Stg,
        CfgDeployEnv::Prod => IdentityEnv::Prod,
    }
}

struct TelemetryRuntime {
    identity: ServiceIdentity,
    tracer_provider: SdkTracerProvider,
    meter_provider: Option<SdkMeterProvider>,
    latency_histogram: Option<Histogram<f64>>,
    hook_counter: Option<Counter<u64>>,
    latency_base_attributes: Vec<KeyValue>,
    service_label: String,
    env_label: String,
    version_label: String,
    prometheus: Option<PrometheusState>,
}

struct PrometheusState {
    exporter: Arc<PrometheusExporter>,
    task: JoinHandle<()>,
}
impl TelemetryRuntime {
    async fn initialize(cfg: TelemetryConfig, identity: ServiceIdentity) -> Result<Self> {
        let resource = Resource::builder()
            .with_attributes(
                identity
                    .resource_pairs()
                    .into_iter()
                    .map(|(k, v)| KeyValue::new(k.to_string(), v))
                    .collect::<Vec<_>>(),
            )
            .build();

        let level = cfg.level;
        let mut tracer_builder = SdkTracerProvider::builder().with_resource(resource.clone());
        if level != ObsLevel::Off {
            if let Some(endpoint) = cfg.otlp_endpoint.as_deref() {
                let exporter = SpanExporter::builder()
                    .with_http()
                    .with_endpoint(endpoint)
                    .build()
                    .context("failed to build OTLP span exporter")?;
                tracer_builder = tracer_builder.with_batch_exporter(exporter);
            }
        }

        let tracer_provider = tracer_builder.build();
        let tracer = tracer_provider.tracer("obs_demo");

        global::set_text_map_propagator(TraceContextPropagator::new());
        global::set_tracer_provider(tracer_provider.clone());

        let otel_layer = if level == ObsLevel::Off {
            None
        } else {
            Some(tracing_opentelemetry::layer().with_tracer(tracer))
        };

        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(cfg.log_level.clone()));
        let fmt_layer = fmt::layer()
            .event_format(JsonLogFormatter::new(&identity))
            .with_ansi(false);

        let subscriber = Registry::default()
            .with(filter)
            .with(otel_layer)
            .with(fmt_layer);
        tracing::subscriber::set_global_default(subscriber)
            .context("failed to install tracing subscriber")?;

        if level == ObsLevel::Off {
            warn!("observability level is OFF — exporters remain disabled");
        } else if cfg.otlp_endpoint.is_none() {
            warn!(
                "OBSERVABILITY_LEVEL={:?} but OTLP endpoint missing; exporters will remain local",
                level
            );
        }

        let meter_provider = if level == ObsLevel::Off {
            None
        } else if let Some(endpoint) = cfg.otlp_endpoint.as_deref() {
            let exporter = MetricExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .build()
                .context("failed to build OTLP metric exporter")?;
            let reader = PeriodicReader::builder(exporter)
                .with_interval(Duration::from_secs(10))
                .build();
            Some(
                SdkMeterProvider::builder()
                    .with_resource(resource.clone())
                    .with_reader(reader)
                    .build(),
            )
        } else {
            None
        };

        if let Some(provider) = &meter_provider {
            global::set_meter_provider(provider.clone());
        }

        let (latency_histogram, hook_counter) = meter_provider
            .as_ref()
            .map(register_instruments)
            .unwrap_or_default();

        let service_label = identity.service_name.clone();
        let env_label = identity.deploy_env.to_string();
        let version_label = identity.service_version.clone();
        let latency_base_attributes = vec![
            KeyValue::new("service", service_label.clone()),
            KeyValue::new("env", env_label.clone()),
            KeyValue::new("version", version_label.clone()),
        ];

        let prometheus = if cfg.prom_scrape {
            let addr: SocketAddr = cfg
                .metrics_http_addr
                .parse()
                .context("invalid METRICS_HTTP_ADDR; expected host:port")?;
            let exporter = Arc::new(PrometheusExporter::new());
            let task = spawn_prometheus_server(exporter.clone(), addr)
                .await
                .context("failed to bind Prometheus metrics listener")?;
            println!("Prometheus exporter listening at http://{addr}/metrics");
            Some(PrometheusState { exporter, task })
        } else {
            None
        };

        Ok(Self {
            identity,
            tracer_provider,
            meter_provider,
            latency_histogram,
            hook_counter,
            latency_base_attributes,
            service_label,
            env_label,
            version_label,
            prometheus,
        })
    }

    fn latency_guard<'a>(&'a self, op: &'a str) -> LatencyGuard<'a> {
        LatencyGuard {
            runtime: self,
            op,
            start: Instant::now(),
        }
    }

    fn record_latency(&self, op: &str, duration: Duration) {
        let seconds = duration.as_secs_f64();
        if let Some(histogram) = &self.latency_histogram {
            let mut attrs = self.latency_base_attributes.clone();
            attrs.push(KeyValue::new("op", op.to_string()));
            histogram.record(seconds, &attrs);
        }
        if let Some(prom) = &self.prometheus {
            prom.exporter.record_latency(
                op,
                &self.service_label,
                &self.env_label,
                &self.version_label,
                seconds,
            );
        }
    }

    fn record_hook(&self, hook_id: &str, status: HookStatus) {
        if let Some(counter) = &self.hook_counter {
            let attrs = [
                KeyValue::new("hook_id", hook_id.to_string()),
                KeyValue::new("status", status.as_str().to_string()),
            ];
            counter.add(1, &attrs);
        }
        if let Some(prom) = &self.prometheus {
            prom.exporter.record_hook(hook_id, status.as_str());
        }
    }

    async fn shutdown(&self) {
        if let Some(state) = &self.prometheus {
            state.task.abort();
        }
        if let Some(provider) = &self.meter_provider {
            let _ = provider.force_flush();
            let _ = provider.shutdown();
        }
        let _ = self.tracer_provider.force_flush();
        let _ = self.tracer_provider.shutdown();
    }
}

fn register_instruments(
    provider: &SdkMeterProvider,
) -> (Option<Histogram<f64>>, Option<Counter<u64>>) {
    let meter: Meter = provider.meter("obs_demo");
    let histogram = meter
        .f64_histogram(contract::METRIC_AMM_OP_LATENCY_SECONDS)
        .with_description("Synthetic AMM operation latency in seconds")
        .with_unit("s")
        .with_boundaries(contract::AMM_OP_LATENCY_BUCKETS.to_vec())
        .build();
    let hook_counter = meter
        .u64_counter(contract::METRIC_HOOK_EXECUTIONS_TOTAL)
        .with_description("Synthetic hook execution counter")
        .build();
    (Some(histogram), Some(hook_counter))
}

struct LatencyGuard<'a> {
    runtime: &'a TelemetryRuntime,
    op: &'a str,
    start: Instant,
}

impl LatencyGuard<'_> {
    fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
}

impl Drop for LatencyGuard<'_> {
    fn drop(&mut self) {
        self.runtime.record_latency(self.op, self.start.elapsed());
    }
}

struct PrometheusExporter {
    latencies: Mutex<HashMap<LatencyKey, Vec<f64>>>,
    hooks: Mutex<HashMap<HookKey, u64>>,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct LatencyKey {
    op: String,
    service: String,
    env: String,
    version: String,
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
struct HookKey {
    hook_id: String,
    status: String,
}

impl PrometheusExporter {
    fn new() -> Self {
        Self {
            latencies: Mutex::new(HashMap::new()),
            hooks: Mutex::new(HashMap::new()),
        }
    }

    fn record_latency(&self, op: &str, service: &str, env: &str, version: &str, value: f64) {
        let mut guard = self
            .latencies
            .lock()
            .expect("prometheus latency registry poisoned");
        let key = LatencyKey {
            op: op.to_string(),
            service: service.to_string(),
            env: env.to_string(),
            version: version.to_string(),
        };
        guard.entry(key).or_default().push(value);
    }

    fn record_hook(&self, hook_id: &str, status: &str) {
        let mut guard = self
            .hooks
            .lock()
            .expect("prometheus hook registry poisoned");
        let key = HookKey {
            hook_id: hook_id.to_string(),
            status: status.to_string(),
        };
        *guard.entry(key).or_insert(0) += 1;
    }

    fn render(&self) -> String {
        let mut buffer = String::new();
        buffer.push_str("# HELP amm_op_latency_seconds Latency per AMM operation in seconds\n");
        buffer.push_str("# TYPE amm_op_latency_seconds histogram\n");

        let latencies = self
            .latencies
            .lock()
            .expect("prometheus latency registry poisoned")
            .clone();
        let mut latency_keys: Vec<_> = latencies.keys().cloned().collect();
        latency_keys.sort_by(|a, b| {
            a.service
                .cmp(&b.service)
                .then(a.env.cmp(&b.env))
                .then(a.version.cmp(&b.version))
                .then(a.op.cmp(&b.op))
        });

        for key in latency_keys {
            if let Some(values) = latencies.get(&key) {
                render_histogram(&mut buffer, &key, values);
            }
        }

        buffer.push_str("# HELP hook_executions_total Hook executions by hook_id and status\n");
        buffer.push_str("# TYPE hook_executions_total counter\n");
        let hooks = self
            .hooks
            .lock()
            .expect("prometheus hook registry poisoned")
            .clone();
        let mut hook_keys: Vec<_> = hooks.keys().cloned().collect();
        hook_keys.sort_by(|a, b| a.hook_id.cmp(&b.hook_id).then(a.status.cmp(&b.status)));
        for key in hook_keys {
            if let Some(value) = hooks.get(&key) {
                let labels = format!(
                    "{{hook_id=\"{}\",status=\"{}\"}}",
                    escape_label_value(&key.hook_id),
                    escape_label_value(&key.status)
                );
                buffer.push_str(&format!("hook_executions_total{} {}\n", labels, value));
            }
        }

        buffer
    }
}
fn render_histogram(buffer: &mut String, key: &LatencyKey, samples: &[f64]) {
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    for &bucket in contract::AMM_OP_LATENCY_BUCKETS {
        let cumulative = sorted.iter().filter(|value| **value <= bucket).count() as u64;
        let labels = format!(
            "{{op=\"{}\",service=\"{}\",env=\"{}\",version=\"{}\",le=\"{}\"}}",
            escape_label_value(&key.op),
            escape_label_value(&key.service),
            escape_label_value(&key.env),
            escape_label_value(&key.version),
            format_bucket(bucket)
        );
        buffer.push_str(&format!(
            "amm_op_latency_seconds_bucket{} {}\n",
            labels, cumulative
        ));
    }

    let labels_inf = format!(
        "{{op=\"{}\",service=\"{}\",env=\"{}\",version=\"{}\",le=\"+Inf\"}}",
        escape_label_value(&key.op),
        escape_label_value(&key.service),
        escape_label_value(&key.env),
        escape_label_value(&key.version)
    );
    buffer.push_str(&format!(
        "amm_op_latency_seconds_bucket{} {}\n",
        labels_inf,
        sorted.len()
    ));

    let base_labels = format!(
        "{{op=\"{}\",service=\"{}\",env=\"{}\",version=\"{}\"}}",
        escape_label_value(&key.op),
        escape_label_value(&key.service),
        escape_label_value(&key.env),
        escape_label_value(&key.version)
    );
    let sum: f64 = sorted.iter().copied().sum();
    buffer.push_str(&format!(
        "amm_op_latency_seconds_sum{} {}\n",
        base_labels, sum
    ));
    buffer.push_str(&format!(
        "amm_op_latency_seconds_count{} {}\n",
        base_labels,
        sorted.len()
    ));
}

fn escape_label_value(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            other => escaped.push(other),
        }
    }
    escaped
}

fn format_bucket(value: f64) -> String {
    let formatted = format!("{:.6}", value);
    let trimmed = formatted.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

async fn spawn_prometheus_server(
    exporter: Arc<PrometheusExporter>,
    addr: SocketAddr,
) -> Result<JoinHandle<()>> {
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|err| anyhow!("failed to bind {}: {}", addr, err))?;
    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let exporter = exporter.clone();
                    tokio::spawn(async move {
                        if let Err(err) = respond_with_metrics(stream, exporter).await {
                            eprintln!("obs_demo: metrics connection error: {}", err);
                        }
                    });
                }
                Err(err) => {
                    eprintln!("obs_demo: failed to accept metrics connection: {}", err);
                    break;
                }
            }
        }
    });
    Ok(handle)
}

async fn respond_with_metrics(
    mut stream: TcpStream,
    exporter: Arc<PrometheusExporter>,
) -> std::io::Result<()> {
    let mut buffer = [0_u8; 1024];
    let _ = stream.read(&mut buffer).await?;
    let body = exporter.render();
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
        body.len(), body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await
}

#[derive(Default, Clone, Copy)]
struct OperationSummary {
    swap: usize,
    pricing: usize,
    cdc: usize,
}

struct SyntheticWorkload {
    runtime: Arc<TelemetryRuntime>,
    rng: DeterministicRng,
    base_reserve: f64,
    quote_reserve: f64,
    liquidity_scale: f64,
    cdc_offsets: HashMap<(String, String), u64>,
    summary: OperationSummary,
}

impl SyntheticWorkload {
    fn new(runtime: Arc<TelemetryRuntime>) -> Self {
        Self {
            runtime,
            rng: DeterministicRng::new(0xC0FFEE123456789),
            base_reserve: 1_200.0,
            quote_reserve: 800.0,
            liquidity_scale: 1.0,
            cdc_offsets: HashMap::new(),
            summary: OperationSummary::default(),
        }
    }

    async fn execute(&mut self, step: usize) -> Result<()> {
        match step % 3 {
            0 => self.perform_swap().await?,
            1 => self.produce_quote().await?,
            _ => self.consume_stream().await?,
        }
        Ok(())
    }

    fn summary(&self) -> OperationSummary {
        self.summary
    }

    async fn perform_swap(&mut self) -> Result<()> {
        let event = self.generate_swap_event();
        let span = tracing::info_span!(
            contract::SPAN_AMM_SWAP,
            op = SWAP_OP,
            "amm.k_before" = event.k_before,
            "amm.k_after" = event.k_after,
            "amm.delta_k_ratio" = event.delta_k_ratio,
            "amm.fee_ppm" = event.fee_ppm,
            "amm.input" = event.input,
            "amm.output" = event.output,
            "amm.direction" = event.direction.as_str(),
            "service.name" = %self.runtime.identity.service_name,
            "service.version" = %self.runtime.identity.service_version,
            "deployment.environment" = %self.runtime.identity.deploy_env,
        );
        let _entered = span.enter();
        let guard = self.runtime.latency_guard(SWAP_OP);
        let processing_ms = self.rng.next_range(35.0, 95.0);
        sleep(Duration::from_millis(processing_ms as u64)).await;
        let latency = guard.elapsed();
        drop(guard);

        let hook_status = if self.rng.next_range(0.0, 1.0) < 0.92 {
            HookStatus::Success
        } else {
            HookStatus::Error
        };
        self.runtime.record_hook("amm.risk-check", hook_status);

        info!(
            op = SWAP_OP,
            "latency_seconds" = latency.as_secs_f64(),
            "amm.k_before" = event.k_before,
            "amm.k_after" = event.k_after,
            "amm.delta_k_ratio" = event.delta_k_ratio,
            "amm.fee_ppm" = event.fee_ppm,
            "amm.input" = event.input,
            "amm.output" = event.output,
            "amm.direction" = event.direction.as_str(),
            "msg" = "swap executed",
        );

        self.summary.swap += 1;
        Ok(())
    }

    async fn produce_quote(&mut self) -> Result<()> {
        let event = self.generate_quote_event();
        let span = tracing::info_span!(
            contract::SPAN_PRICING_QUOTE,
            op = PRICING_OP,
            "amm.k_before" = event.k_before,
            "amm.k_after" = event.k_after,
            "amm.delta_k_ratio" = event.delta_k_ratio,
            "amm.fee_ppm" = event.fee_ppm,
            "amm.input" = event.input,
            "amm.output" = event.output,
            "pricing.mid_price" = event.mid_price,
            "service.name" = %self.runtime.identity.service_name,
            "service.version" = %self.runtime.identity.service_version,
            "deployment.environment" = %self.runtime.identity.deploy_env,
        );
        let _entered = span.enter();
        let guard = self.runtime.latency_guard(PRICING_OP);
        let processing_ms = self.rng.next_range(20.0, 70.0);
        sleep(Duration::from_millis(processing_ms as u64)).await;
        let latency = guard.elapsed();
        drop(guard);

        info!(
            op = PRICING_OP,
            "latency_seconds" = latency.as_secs_f64(),
            "amm.k_before" = event.k_before,
            "amm.k_after" = event.k_after,
            "amm.delta_k_ratio" = event.delta_k_ratio,
            "amm.fee_ppm" = event.fee_ppm,
            "amm.input" = event.input,
            "amm.output" = event.output,
            "pricing.mid_price" = event.mid_price,
            "msg" = "pricing quote generated",
        );

        self.summary.pricing += 1;
        Ok(())
    }

    async fn consume_stream(&mut self) -> Result<()> {
        let event = self.generate_cdc_event();
        let span = tracing::info_span!(
            contract::SPAN_CDC_CONSUME,
            op = CDC_OP,
            "amm.k_before" = event.k_before,
            "amm.k_after" = event.k_after,
            "amm.delta_k_ratio" = event.delta_k_ratio,
            "amm.fee_ppm" = event.fee_ppm,
            "amm.input" = event.input,
            "amm.output" = event.output,
            "cdc.stream" = event.stream.as_str(),
            "cdc.partition" = event.partition.as_str(),
            "cdc.offset_before" = event.offset_before,
            "cdc.offset_after" = event.offset_after,
            "cdc.records" = event.records,
            "cdc.lag_seconds" = event.lag_seconds,
            "service.name" = %self.runtime.identity.service_name,
            "service.version" = %self.runtime.identity.service_version,
            "deployment.environment" = %self.runtime.identity.deploy_env,
        );
        let _entered = span.enter();
        let guard = self.runtime.latency_guard(CDC_OP);
        let processing_ms = self.rng.next_range(40.0, 110.0);
        sleep(Duration::from_millis(processing_ms as u64)).await;
        let latency = guard.elapsed();
        drop(guard);

        info!(
            op = CDC_OP,
            "latency_seconds" = latency.as_secs_f64(),
            "amm.k_before" = event.k_before,
            "amm.k_after" = event.k_after,
            "amm.delta_k_ratio" = event.delta_k_ratio,
            "amm.fee_ppm" = event.fee_ppm,
            "amm.input" = event.input,
            "amm.output" = event.output,
            "cdc.stream" = event.stream.as_str(),
            "cdc.partition" = event.partition.as_str(),
            "cdc.offset_before" = event.offset_before,
            "cdc.offset_after" = event.offset_after,
            "cdc.records" = event.records,
            "cdc.lag_seconds" = event.lag_seconds,
            "msg" = "cdc batch consumed",
        );

        self.summary.cdc += 1;
        Ok(())
    }

    fn generate_swap_event(&mut self) -> SwapEvent {
        let reserve_a = self.base_reserve * self.liquidity_scale;
        let reserve_b = self.quote_reserve * self.liquidity_scale;
        let k_before = reserve_a * reserve_b;
        let fee_ppm = self.rng.next_i32(100, 500);
        let direction = if self.rng.next_range(0.0, 1.0) < 0.5 {
            SwapDirection::BaseToQuote
        } else {
            SwapDirection::QuoteToBase
        };
        let trade_amount = self.rng.next_range(10.0, 120.0);
        let fee_multiplier = 1.0 - (fee_ppm as f64 / 1_000_000.0);
        let epsilon = self.rng.next_range(-0.05, 0.05);

        let (input, output, new_reserve_a, new_reserve_b) = match direction {
            SwapDirection::BaseToQuote => {
                let effective_input = trade_amount * fee_multiplier;
                let new_reserve_a = reserve_a + effective_input;
                let new_reserve_b = k_before / new_reserve_a;
                let output = (reserve_b - new_reserve_b).max(0.0);
                (trade_amount, output, new_reserve_a, new_reserve_b)
            }
            SwapDirection::QuoteToBase => {
                let effective_input = trade_amount * fee_multiplier;
                let new_reserve_b = reserve_b + effective_input;
                let new_reserve_a = k_before / new_reserve_b;
                let output = (reserve_a - new_reserve_a).max(0.0);
                (trade_amount, output, new_reserve_a, new_reserve_b)
            }
        };

        // Update underlying reserves before applying liquidity scaling.
        self.base_reserve = new_reserve_a / self.liquidity_scale;
        self.quote_reserve = new_reserve_b / self.liquidity_scale;

        let scale_multiplier = (1.0 + epsilon).max(0.95).sqrt();
        self.liquidity_scale *= scale_multiplier;
        let k_after = (self.base_reserve * self.liquidity_scale)
            * (self.quote_reserve * self.liquidity_scale);
        let delta_k_ratio = if k_before > 0.0 {
            (k_after - k_before) / k_before
        } else {
            0.0
        };
        let invariant_input = input.max(0.0);
        let invariant_output = output.max(0.0);

        SwapEvent {
            k_before,
            k_after,
            delta_k_ratio,
            fee_ppm,
            input: invariant_input,
            output: invariant_output,
            direction,
        }
    }

    fn generate_quote_event(&mut self) -> QuoteEvent {
        let reserve_a = self.base_reserve * self.liquidity_scale;
        let reserve_b = self.quote_reserve * self.liquidity_scale;
        let k_before = reserve_a * reserve_b;
        let fee_ppm = self.rng.next_i32(100, 500);
        let direction = if self.rng.next_range(0.0, 1.0) < 0.5 {
            SwapDirection::BaseToQuote
        } else {
            SwapDirection::QuoteToBase
        };
        let amount = self.rng.next_range(5.0, 80.0);
        let fee_multiplier = 1.0 - (fee_ppm as f64 / 1_000_000.0);
        let effective = amount * fee_multiplier;

        let (input, output, mid_price) = match direction {
            SwapDirection::BaseToQuote => {
                let new_reserve_a = reserve_a + effective;
                let new_reserve_b = k_before / new_reserve_a;
                let output = (reserve_b - new_reserve_b).max(0.0);
                let price = new_reserve_b / new_reserve_a;
                (amount, output, price)
            }
            SwapDirection::QuoteToBase => {
                let new_reserve_b = reserve_b + effective;
                let new_reserve_a = k_before / new_reserve_b;
                let output = (reserve_a - new_reserve_a).max(0.0);
                let price = new_reserve_b / new_reserve_a;
                (amount, output, price)
            }
        };

        let epsilon = self.rng.next_range(-0.02, 0.02);
        let scale_multiplier = (1.0 + epsilon).max(0.98).sqrt();
        self.liquidity_scale *= scale_multiplier;
        let k_after = (self.base_reserve * self.liquidity_scale)
            * (self.quote_reserve * self.liquidity_scale);
        let delta_k_ratio = if k_before > 0.0 {
            (k_after - k_before) / k_before
        } else {
            0.0
        };

        QuoteEvent {
            k_before,
            k_after,
            delta_k_ratio,
            fee_ppm,
            input,
            output,
            mid_price,
        }
    }

    fn generate_cdc_event(&mut self) -> CdcEvent {
        let streams = ["trades", "quotes"];
        let partitions = ["p0", "p1"];
        let stream = streams
            [(self.rng.next_range(0.0, 1.0) * 2.0).floor() as usize % streams.len()]
        .to_string();
        let partition = partitions
            [(self.rng.next_range(0.0, 1.0) * 2.0).floor() as usize % partitions.len()]
        .to_string();
        let key = (stream.clone(), partition.clone());
        let offset_before = *self.cdc_offsets.get(&key).unwrap_or(&0);
        let records = self.rng.next_i32(1, 50) as u64;
        let offset_after = offset_before + records;
        self.cdc_offsets.insert(key, offset_after);

        let reserve_a = self.base_reserve * self.liquidity_scale;
        let reserve_b = self.quote_reserve * self.liquidity_scale;
        let k_before = reserve_a * reserve_b;
        let fee_ppm = self.rng.next_i32(100, 500);
        let epsilon = self.rng.next_range(-0.01, 0.01);
        let scale_multiplier = (1.0 + epsilon).max(0.99).sqrt();
        self.liquidity_scale *= scale_multiplier;
        let k_after = (self.base_reserve * self.liquidity_scale)
            * (self.quote_reserve * self.liquidity_scale);
        let delta_k_ratio = if k_before > 0.0 {
            (k_after - k_before) / k_before
        } else {
            0.0
        };

        let lag_seconds = self.rng.next_range(0.0, 5.0);

        CdcEvent {
            k_before,
            k_after,
            delta_k_ratio,
            fee_ppm,
            input: records as f64,
            output: 0.0,
            stream,
            partition,
            offset_before,
            offset_after,
            records,
            lag_seconds,
        }
    }
}
#[derive(Clone, Copy)]
enum SwapDirection {
    BaseToQuote,
    QuoteToBase,
}

impl SwapDirection {
    fn as_str(self) -> &'static str {
        match self {
            SwapDirection::BaseToQuote => "base_to_quote",
            SwapDirection::QuoteToBase => "quote_to_base",
        }
    }
}

struct SwapEvent {
    k_before: f64,
    k_after: f64,
    delta_k_ratio: f64,
    fee_ppm: i32,
    input: f64,
    output: f64,
    direction: SwapDirection,
}

struct QuoteEvent {
    k_before: f64,
    k_after: f64,
    delta_k_ratio: f64,
    fee_ppm: i32,
    input: f64,
    output: f64,
    mid_price: f64,
}

struct CdcEvent {
    k_before: f64,
    k_after: f64,
    delta_k_ratio: f64,
    fee_ppm: i32,
    input: f64,
    output: f64,
    stream: String,
    partition: String,
    offset_before: u64,
    offset_after: u64,
    records: u64,
    lag_seconds: f64,
}

struct DeterministicRng {
    state: u64,
}

impl DeterministicRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_range(&mut self, min: f64, max: f64) -> f64 {
        let rand = (self.next_u64() >> 11) as f64 / ((1_u64 << 53) as f64);
        min + (max - min) * rand
    }

    fn next_i32(&mut self, min: i32, max: i32) -> i32 {
        let span = (max - min + 1) as u64;
        min + (self.next_u64() % span) as i32
    }
}

#[derive(Clone, Copy)]
enum HookStatus {
    Success,
    Error,
}

impl HookStatus {
    fn as_str(self) -> &'static str {
        match self {
            HookStatus::Success => "success",
            HookStatus::Error => "error",
        }
    }
}

struct JsonLogFormatter {
    service: String,
    env: String,
    version: String,
}

impl JsonLogFormatter {
    fn new(identity: &ServiceIdentity) -> Self {
        Self {
            service: identity.service_name.clone(),
            env: identity.deploy_env.to_string(),
            version: identity.service_version.clone(),
        }
    }
}

impl<S, N> FormatEvent<S, N> for JsonLogFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let mut visitor = LogEventVisitor::default();
        event.record(&mut visitor);

        let message = visitor
            .message
            .unwrap_or_else(|| event.metadata().target().to_string());
        let op = visitor.op.unwrap_or_else(|| "unknown".to_string());
        let level = event.metadata().level().as_str().to_lowercase();

        let span_context = Span::current().context();
        let otel_span = span_context.span().span_context().clone();
        let (trace_id, span_id) = if otel_span.is_valid() {
            (
                otel_span.trace_id().to_string(),
                otel_span.span_id().to_string(),
            )
        } else {
            (
                "00000000000000000000000000000000".to_string(),
                "0000000000000000".to_string(),
            )
        };

        let timestamp = format_timestamp();

        let mut log = JsonMap::new();
        log.insert("ts".to_string(), JsonValue::String(timestamp));
        log.insert("level".to_string(), JsonValue::String(level));
        log.insert("msg".to_string(), JsonValue::String(message));
        log.insert("trace_id".to_string(), JsonValue::String(trace_id));
        log.insert("span_id".to_string(), JsonValue::String(span_id));
        log.insert(
            "service".to_string(),
            JsonValue::String(self.service.clone()),
        );
        log.insert("env".to_string(), JsonValue::String(self.env.clone()));
        log.insert(
            "version".to_string(),
            JsonValue::String(self.version.clone()),
        );
        log.insert("op".to_string(), JsonValue::String(op));

        if let Some(kind) = visitor.error_kind {
            log.insert("error.kind".to_string(), JsonValue::String(kind));
        }
        if let Some(message) = visitor.error_message {
            log.insert("error.message".to_string(), JsonValue::String(message));
        }
        if !visitor.extra.is_empty() {
            log.insert("extra".to_string(), JsonValue::Object(visitor.extra));
        }

        let json = JsonValue::Object(log);
        let serialized = serde_json::to_string(&json).map_err(|_| std::fmt::Error)?;
        write!(writer, "{}\n", serialized)
    }
}

#[derive(Default)]
struct LogEventVisitor {
    message: Option<String>,
    op: Option<String>,
    error_kind: Option<String>,
    error_message: Option<String>,
    extra: JsonMap<String, JsonValue>,
}

impl tracing::field::Visit for LogEventVisitor {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        match field.name() {
            "message" | "msg" => self.message = Some(value.to_string()),
            "op" => self.op = Some(value.to_string()),
            "error.kind" => self.error_kind = Some(value.to_string()),
            "error.message" => self.error_message = Some(value.to_string()),
            other => insert_extra(&mut self.extra, other, JsonValue::String(value.to_string())),
        }
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        insert_extra(&mut self.extra, field.name(), JsonValue::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        insert_extra(&mut self.extra, field.name(), JsonValue::from(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        insert_extra(&mut self.extra, field.name(), JsonValue::from(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        insert_extra(&mut self.extra, field.name(), JsonValue::from(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        insert_extra(
            &mut self.extra,
            field.name(),
            JsonValue::String(format!("{:?}", value)),
        );
    }
}

fn insert_extra(target: &mut JsonMap<String, JsonValue>, key: &str, value: JsonValue) {
    if key.is_empty() {
        return;
    }
    if let Some((head, tail)) = key.split_once('.') {
        let entry = target
            .entry(head.to_string())
            .or_insert_with(|| JsonValue::Object(JsonMap::new()));
        if let JsonValue::Object(ref mut child) = entry {
            insert_extra(child, tail, value);
        } else {
            let mut map = JsonMap::new();
            insert_extra(&mut map, tail, value);
            *entry = JsonValue::Object(map);
        }
    } else {
        target.insert(key.to_string(), value);
    }
}

fn format_timestamp() -> String {
    let now = SystemTime::now();
    let duration = now.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();
    let (year, month, day, hour, minute, second) = unix_seconds_to_datetime(secs);
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z",
        year = year,
        month = month,
        day = day,
        hour = hour,
        minute = minute,
        second = second
    )
}

fn unix_seconds_to_datetime(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    const SECS_PER_MINUTE: u64 = 60;
    const SECS_PER_HOUR: u64 = 60 * SECS_PER_MINUTE;
    const SECS_PER_DAY: u64 = 24 * SECS_PER_HOUR;

    let mut days = secs / SECS_PER_DAY;
    let mut rem = secs % SECS_PER_DAY;
    let hour = (rem / SECS_PER_HOUR) as u32;
    rem %= SECS_PER_HOUR;
    let minute = (rem / SECS_PER_MINUTE) as u32;
    let second = (rem % SECS_PER_MINUTE) as u32;

    let mut year = 1970;
    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 } as u64;
        if days >= days_in_year {
            days -= days_in_year;
            year += 1;
        } else {
            break;
        }
    }

    let mut month_lengths = [31_u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_leap_year(year) {
        month_lengths[1] = 29;
    }
    let mut month = 1_u32;
    for length in month_lengths.iter() {
        if days >= *length {
            days -= *length;
            month += 1;
        } else {
            break;
        }
    }
    let day = (days + 1) as u32;
    (year, month, day, hour, minute, second)
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
