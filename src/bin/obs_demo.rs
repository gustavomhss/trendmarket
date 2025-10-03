use anyhow::{Context, Result};
use std::env;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use credit_engine_core::telemetry_contract::AMM_OP_LATENCY_BUCKETS;

const SERVICE_NAME: &str = "ce-amm";
const DEFAULT_METRICS_ADDR: &str = "127.0.0.1:9464";
const DEFAULT_OPS: u32 = 10;
const DEFAULT_ENV: &str = "dev";

#[derive(Clone)]
struct MetricsState {
    buckets: Vec<(f64, u64)>,
    sum: f64,
    count: u64,
    hook_count: u64,
    service: String,
    env: String,
    version: String,
}

impl MetricsState {
    fn new(service: String, env: String, version: String) -> Self {
        Self {
            buckets: AMM_OP_LATENCY_BUCKETS
                .iter()
                .copied()
                .map(|b| (b, 0))
                .collect(),
            sum: 0.0,
            count: 0,
            hook_count: 0,
            service,
            env,
            version,
        }
    }

    fn record_latency(&mut self, seconds: f64) {
        self.sum += seconds;
        self.count += 1;
        for (upper, bucket_count) in &mut self.buckets {
            if seconds <= *upper {
                *bucket_count += 1;
            }
        }
    }

    fn increment_hook(&mut self) {
        self.hook_count += 1;
    }

    fn render(&self) -> String {
        let mut output = String::new();
        for (upper, bucket_count) in &self.buckets {
            output.push_str(&format!(
                "amm_op_latency_seconds_bucket{{op=\"swap\",service=\"{}\",env=\"{}\",version=\"{}\",le=\"{:.3}\"}} {}\n",
                self.service, self.env, self.version, upper, bucket_count
            ));
        }
        output.push_str(&format!(
            "amm_op_latency_seconds_bucket{{op=\"swap\",service=\"{}\",env=\"{}\",version=\"{}\",le=\"+Inf\"}} {}\n",
            self.service, self.env, self.version, self.count
        ));
        output.push_str(&format!(
            "amm_op_latency_seconds_sum{{op=\"swap\",service=\"{}\",env=\"{}\",version=\"{}\"}} {:.6}\n",
            self.service, self.env, self.version, self.sum
        ));
        output.push_str(&format!(
            "amm_op_latency_seconds_count{{op=\"swap\",service=\"{}\",env=\"{}\",version=\"{}\"}} {}\n",
            self.service, self.env, self.version, self.count
        ));
        output.push_str(&format!(
            "hook_executions_total{{service=\"{}\",env=\"{}\",version=\"{}\",hook_id=\"swap-latency\",status=\"success\"}} {}\n",
            self.service, self.env, self.version, self.hook_count
        ));
        output
    }
}

fn start_metrics_server(
    addr: &str,
    state: Arc<Mutex<MetricsState>>,
    shutdown: Arc<AtomicBool>,
) -> Result<thread::JoinHandle<()>> {
    let listener = TcpListener::bind(addr)
        .with_context(|| format!("failed to bind prometheus listener at {addr}"))?;
    listener
        .set_nonblocking(true)
        .context("failed to configure non-blocking listener")?;

    let handle = thread::spawn(move || {
        while !shutdown.load(Ordering::Acquire) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let mut buf = [0u8; 512];
                    let _ = stream.read(&mut buf);
                    let body = {
                        let guard = state.lock().expect("metrics mutex poisoned");
                        guard.render()
                    };
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(), body
                    );
                    let _ = stream.write_all(response.as_bytes());
                }
                Err(err) => {
                    if err.kind() == std::io::ErrorKind::WouldBlock {
                        thread::sleep(Duration::from_millis(20));
                        continue;
                    }
                    break;
                }
            }
        }
    });

    Ok(handle)
}

fn parse_ops() -> u32 {
    let mut from_cli = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--ops" {
            if let Some(value) = args.next() {
                if let Ok(parsed) = value.parse::<u32>() {
                    from_cli = Some(parsed);
                }
            }
        }
    }

    if let Some(value) = from_cli {
        return value.max(1);
    }

    if let Ok(env_value) = env::var("OBS_DEMO_OPS") {
        if let Ok(parsed) = env_value.parse::<u32>() {
            return parsed.max(1);
        }
    }

    DEFAULT_OPS
}

fn now_timestamp() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0));
    let mut days = duration.as_secs() / 86_400;
    let seconds_of_day = duration.as_secs() % 86_400;

    let hour = (seconds_of_day / 3_600) as u32;
    let minute = ((seconds_of_day % 3_600) / 60) as u32;
    let second = (seconds_of_day % 60) as u32;

    let mut year: i32 = 1970;
    loop {
        let leap = is_leap(year);
        let year_days = if leap { 366 } else { 365 };
        if days >= year_days {
            days -= year_days;
            year += 1;
        } else {
            break;
        }
    }

    let leap = is_leap(year);
    let mut month = 1;
    let month_lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (idx, length) in month_lengths.iter().enumerate() {
        let len = if leap && idx == 1 { 29 } else { *length } as u64;
        if days >= len {
            days -= len;
            month += 1;
        } else {
            break;
        }
    }
    let day = (days + 1) as u32;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn synthetic_latency(index: u32) -> f64 {
    const PATTERN_MS: [f64; 5] = [12.0, 38.0, 82.0, 190.0, 320.0];
    let ms = PATTERN_MS[(index as usize) % PATTERN_MS.len()];
    ms / 1000.0
}

fn make_trace_id(base_nanos: u128, index: u32) -> String {
    let value = base_nanos.wrapping_add(index as u128 * 97);
    format!("{:032x}", value & 0xffffffffffffffffffffffffffffffff)
}

fn make_span_id(base_nanos: u128, index: u32) -> String {
    let value = (base_nanos >> 32).wrapping_add((index as u128) * 131);
    format!("{:016x}", (value as u64) & 0xffffffffffffffff)
}

fn main() -> Result<()> {
    let ops = parse_ops();
    let prom_enabled = env::var("PROM_SCRAPE")
        .unwrap_or_else(|_| "off".to_string())
        .eq_ignore_ascii_case("on");
    let metrics_addr =
        env::var("METRICS_HTTP_ADDR").unwrap_or_else(|_| DEFAULT_METRICS_ADDR.to_string());
    let deploy_env = env::var("DEPLOY_ENV").unwrap_or_else(|_| DEFAULT_ENV.to_string());
    let version =
        env::var("SERVICE_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());

    let metrics_state = Arc::new(Mutex::new(MetricsState::new(
        SERVICE_NAME.to_string(),
        deploy_env.clone(),
        version.clone(),
    )));
    let shutdown = Arc::new(AtomicBool::new(false));

    let server_handle = if prom_enabled {
        Some(start_metrics_server(
            &metrics_addr,
            Arc::clone(&metrics_state),
            Arc::clone(&shutdown),
        )?)
    } else {
        None
    };

    let base_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_nanos();

    for idx in 0..ops {
        let latency_seconds = synthetic_latency(idx);
        if prom_enabled {
            if let Ok(mut guard) = metrics_state.lock() {
                guard.record_latency(latency_seconds);
                guard.increment_hook();
            }
        }

        let timestamp = now_timestamp();
        let trace_id = make_trace_id(base_nanos, idx);
        let span_id = make_span_id(base_nanos, idx);
        let log_line = format!(
            "{{\"ts\":\"{}\",\"level\":\"info\",\"msg\":\"swap operation executed\",\"service\":\"{}\",\"env\":\"{}\",\"version\":\"{}\",\"op\":\"swap\",\"trace_id\":\"{}\",\"span_id\":\"{}\",\"hook_id\":\"swap-latency\",\"latency_ms\":{:.3}}}",
            timestamp,
            SERVICE_NAME,
            deploy_env,
            version,
            trace_id,
            span_id,
            latency_seconds * 1000.0
        );
        println!("{}", log_line);

        thread::sleep(Duration::from_millis(40));
    }

    if let Some(handle) = server_handle {
        thread::sleep(Duration::from_secs(1));
        shutdown.store(true, Ordering::Release);
        let _ = handle.join();
    }

    Ok(())
}
