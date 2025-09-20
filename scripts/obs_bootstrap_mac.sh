#!/usr/bin/env bash
set -euo pipefail

# 0) Pre-checks
if ! command -v cargo >/dev/null 2>&1; then
  echo "[ERRO] cargo não encontrado. Instale Rustup/Rust." >&2
  exit 1
fi
if ! cargo add --help >/dev/null 2>&1; then
  echo "[INFO] Instalando cargo-edit ..."
  cargo install cargo-edit --locked
fi

# 1) Dependências (adicionadas uma a uma; compatível com cargo-edit antigo)
cargo add anyhow@1 || true
cargo add tracing@0.1 || true
cargo add tracing-subscriber@0.3 || true
cargo add tracing-opentelemetry@0.29 || true
cargo add opentelemetry@0.29 || true
cargo add opentelemetry_sdk@0.29 || true
# recurso http-proto **apenas** no otlp; precisa isolado para não dar erro
cargo add opentelemetry-otlp@0.29 --features http-proto || true
# runtime para o demo opcional
cargo add tokio@1 --features rt-multi-thread,macros || true

# 2) Telemetria (código Rust)
mkdir -p src
cat > src/telemetry.rs <<'RS'
use std::env;
use anyhow::Result;
use opentelemetry::{global, KeyValue};
use opentelemetry::metrics::{Histogram, Meter, Unit};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{Resource, runtime::Tokio, trace as sdktrace, metrics::SdkMeterProvider};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// Handles para shutdown ordenado.
pub struct Telemetry {
    pub tracer_provider: opentelemetry_sdk::trace::SdkTracerProvider,
    pub meter_provider: SdkMeterProvider,
    pub swap_latency_ms: Histogram<f64>,
    pub meter: Meter,
}

impl Telemetry {
    pub fn shutdown(self) {
        let _ = self.tracer_provider.shutdown();
        let _ = self.meter_provider.shutdown();
    }
}

pub fn init(service_name: &str) -> Result<Telemetry> {
    // Resource (atributos que aparecem em métricas e traces)
    let commit_sha = env::var("CE_COMMIT_SHA").unwrap_or_else(|_| "unknown".to_string());
    let service_version = env::var("CE_SERVICE_VERSION")
        .ok()
        .or_else(|| option_env!("CARGO_PKG_VERSION").map(|s| s.to_string()))
        .unwrap_or_else(|| "0.0.0-dev".to_string());
    let deployment_env = env::var("DEPLOY_ENV").unwrap_or_else(|_| "dev".into());

    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.namespace", "credit-engine"),
        KeyValue::new("service.version", service_version),
        KeyValue::new("git.commit.sha", commit_sha.clone()),
        KeyValue::new("deployment.environment", deployment_env),
    ]);

    // Endpoint OTLP (HTTP 4318 por padrão)
    let endpoint = env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());

    // Traces → Collector
    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().http().with_endpoint(endpoint.clone()))
        .with_trace_config(sdktrace::Config::default().with_resource(resource.clone()))
        .install_batch(Tokio)?;

    // Layer de tracing
    let tracer = tracer_provider.tracer(service_name);
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let filter = EnvFilter::try_from_default_env().or_else(|_| EnvFilter::try_new("info"))?;
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_line_number(true)
        .with_thread_ids(false);
    let subscriber = Registry::default().with(filter).with(otel_layer).with(fmt_layer);
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Métricas → Collector → Prometheus/Grafana
    let meter_provider: SdkMeterProvider = opentelemetry_otlp::new_pipeline()
        .metrics(Tokio)
        .with_resource(resource.clone())
        .with_exporter(opentelemetry_otlp::new_exporter().http().with_endpoint(endpoint))
        .build()?;

    global::set_meter_provider(meter_provider.clone());
    let meter = meter_provider.meter(service_name);

    // Instrumentos
    let swap_latency_ms = meter
        .f64_histogram("swap_latency_ms")
        .with_unit(Unit::new("ms"))
        .with_description("Latência de cálculo de swap (engine)")
        .init();

    let _invariant = meter
        .f64_histogram("invariant_error_rel")
        .with_unit(Unit::new("1"))
        .with_description("Erro relativo |Δk/k| em cada operação")
        .init();

    Ok(Telemetry { tracer_provider, meter_provider, swap_latency_ms, meter })
}
RS
