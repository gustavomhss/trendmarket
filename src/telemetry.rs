use anyhow::Result;
use std::time::Duration;

use opentelemetry::{
    global,
    metrics::{Histogram, Meter, MeterProvider},
    trace::TracerProvider as _,
    KeyValue,
};
use opentelemetry_otlp::{MetricExporter, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    resource::Resource,
    trace::SdkTracerProvider,
};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing::Level;

pub struct Telemetry {
    pub tracer_provider: SdkTracerProvider,
    pub meter_provider: SdkMeterProvider,
    pub meter: Meter,
    pub swap_latency_ms: Histogram<f64>,
    pub invariant_error_rel: Histogram<f64>,
}

impl Telemetry {
    pub fn shutdown(&self) {
        let _ = self.meter_provider.force_flush();
        let _ = self.tracer_provider.shutdown();
    }
}

pub fn init(service_name: &str) -> Result<Telemetry> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());

    let commit = std::env::var("CE_COMMIT_SHA").unwrap_or_else(|_| "unknown".into());

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", service_name.to_string()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("git.commit.sha", commit),
        ])
        .build();

    // ---- Traces (OTLP/HTTP) ----
    let span_exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build()?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(span_exporter)
        .build();

    let tracer = tracer_provider.tracer("ce_core");

    // ---- Métricas (OTLP/HTTP) ----
    let metric_exporter = MetricExporter::builder()
        .with_http()
        .with_endpoint(&endpoint)
        .build()?;

    let reader = PeriodicReader::builder(metric_exporter)
        .with_interval(Duration::from_secs(10))
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    // Globais
    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    // tracing -> OTel
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
    let subscriber = Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt_layer)
        .with(otel_layer);
    let _ = tracing::subscriber::set_global_default(subscriber);

    // Instrumentos (histogramas)
    let meter = meter_provider.meter("ce_core");
    let swap_latency_ms = meter
        .f64_histogram("swap_latency_ms")
        .with_unit("ms")
        .with_description("Latency of swap operations in ms")
        .build();
    let invariant_error_rel = meter
        .f64_histogram("invariant_error_rel")
        .with_unit("1")
        .with_description("Relative invariant error |Δk/k| per operation")
        .build();

    Ok(Telemetry { tracer_provider, meter_provider, meter, swap_latency_ms, invariant_error_rel })
}

/// Cria um `Span` INFO com nome **estático** (exigência do tracing) e
/// coloca o nome dinâmico em `span_name`. Inclui `git_commit_sha`.
pub fn make_info_span(name: &str, op_id: u32, component: &str) -> tracing::Span {
    let commit = std::env::var("CE_COMMIT_SHA").unwrap_or_else(|_| "unknown".into());
    tracing::span!(
        target: "ce_core",
        Level::INFO,
        "op",
        git_commit_sha = %commit,
        span_name = %name,
        op_id = op_id,
        component = component
    )
}

