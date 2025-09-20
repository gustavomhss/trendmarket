#!/usr/bin/env bash
set -euo pipefail

if [ -f src/lib.rs ] && grep -q '^pub mod obs;' src/lib.rs; then
  perl -0777 -pe 's/^pub mod obs;$/#[cfg(feature = "obs")]\npub mod obs;/m' -i src/lib.rs
fi

mkdir -p src/obs

cat > src/obs/mod.rs <<'RS'
pub mod init;
pub mod metrics;
pub mod tracingx;
pub mod wrap;
RS

cat > src/obs/init.rs <<'RS'
use anyhow::Result;
use once_cell::sync::OnceCell;
use opentelemetry::{global, KeyValue};
use opentelemetry_prometheus::PrometheusExporter;
use opentelemetry_sdk::{metrics::SdkMeterProvider, Resource};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub(crate) static EXPORTER: OnceCell<PrometheusExporter> = OnceCell::new();

pub fn init(service_name: &str, commit_sha: &str, metrics_addr: &str) -> Result<()> {
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("commit.sha", commit_sha.to_string()),
    ]);

    let exporter = opentelemetry_prometheus::exporter()
        .with_default_histogram_boundaries(vec![
            0.001, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0,
        ])
        .with_resource(resource.clone())
        .build()?;

    let provider = SdkMeterProvider::builder()
        .with_reader(exporter.clone())
        .with_resource(resource.clone())
        .build();
    global::set_meter_provider(provider);

    EXPORTER.set(exporter).ok();

    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name.to_string())
        .install_simple()?;
    let otel_layer = tracing_opentelemetry::OpenTelemetryLayer::new(tracer);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);

    Registry::default()
        .with(filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();

    let _ = crate::obs::metrics::spawn(metrics_addr);
    Ok(())
}
RS

cat > src/obs/metrics.rs <<'RS'
use std::{io::Write, thread};
use anyhow::Result;

pub fn spawn(addr: &str) -> Result<std::thread::JoinHandle<()>> {
    let addr = addr.to_string();
    let handle = thread::spawn(move || {
        let server = tiny_http::Server::http(&addr).expect("metrics server bind");
        for req in server.incoming_requests() {
            if req.url() == "/metrics" {
                let body = encode_prometheus();
                let mut resp = tiny_http::Response::from_data(body);
                let _ = req.respond(resp.with_status_code(200));
            } else {
                let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
            }
        }
    });
    Ok(handle)
}

fn encode_prometheus() -> Vec<u8> {
    let mut buf = Vec::new();
    if let Some(exp) = crate::obs::init::EXPORTER.get() {
        let mf = exp.registry().gather();
        let enc = prometheus::TextEncoder::new();
        let _ = enc.encode(&mf, &mut buf);
    } else {
        let _ = writeln!(&mut buf, "# exporter not ready");
    }
    buf
}
RS

cat > src/obs/tracingx.rs <<'RS'
use tracing_subscriber::EnvFilter;

pub fn filter_from_env() -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))
}
RS

cat > src/obs/wrap.rs <<'RS'
use once_cell::sync::OnceCell;
use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::Histogram;
use std::time::Instant;

static HIST: OnceCell<Histogram<f64>> = OnceCell::new();

fn histogram() -> Histogram<f64> {
    HIST.get_or_init(|| {
        let meter = global::meter("obs.wrap");
        meter.f64_histogram("op_duration_seconds").with_description("operation duration").init()
    }).clone()
}

pub fn time<F, T>(op: &str, f: F) -> T
where
    F: FnOnce() -> T,
{
    let start = Instant::now();
    let out = f();
    let sec = start.elapsed().as_secs_f64();
    histogram().record(sec, &[KeyValue::new("op", op.to_string())]);
    out
}
RS
