use opentelemetry_otlp::WithExportConfig;
use std::time::Duration;
use reqwest::Client;
use opentelemetry::{global, trace::Tracer, KeyValue};
use opentelemetry_sdk::{Resource, trace as sdktrace};
use opentelemetry_otlp::{SpanExporter, MetricExporter, WithHttpConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let traces_endpoint = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318/v1/traces".to_string());
    let metrics_endpoint = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318/v1/metrics".to_string());

    let traces_timeout_ms: u64 = std::env::var("OTEL_EXPORTER_OTLP_TRACES_TIMEOUT").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(5000);
    let metrics_timeout_ms: u64 = std::env::var("OTEL_EXPORTER_OTLP_METRICS_TIMEOUT").ok()
        .and_then(|s| s.parse().ok()).unwrap_or(5000);

    let span_exporter = SpanExporter::builder()
        .with_http()
        .with_http_client(Client::new())
        .with_endpoint(traces_endpoint.clone())
        .with_timeout(Duration::from_millis(traces_timeout_ms))
        .build()?;

    let _metric_exporter = MetricExporter::builder()
        .with_http()
        .with_http_client(Client::new())
        .with_endpoint(metrics_endpoint.clone())
        .with_timeout(Duration::from_millis(metrics_timeout_ms))
        .build()?;

    let tp = sdktrace::SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter)
        .with_resource(Resource::builder().with_attributes(vec![KeyValue::new("service.name", "credit-engine-core")]).build())
        .build();

    global::set_tracer_provider(tp);

    let tracer = global::tracer("otel_http_custom_client_smoke");
    tracer.in_span("test_span_http_custom_client", |_cx| {});

    Ok(())
}
