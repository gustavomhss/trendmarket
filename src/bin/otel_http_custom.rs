use anyhow::{anyhow, Result};
use std::time::Duration;
use reqwest::Client;

use opentelemetry::{global, trace::Tracer, KeyValue};
use opentelemetry_sdk::{
    Resource,
    trace as sdktrace,
    metrics::SdkMeterProvider,
};
use opentelemetry_otlp::{SpanExporter, MetricExporter, WithExportConfig, WithHttpConfig, Protocol};

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum OtlpProtocol { Http, Grpc }

#[tokio::main]
async fn main() -> Result<()> {
    // ===== config (env -> defaults) =====
    let traces_endpoint  = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318/v1/traces".to_string());
    let metrics_endpoint = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318/v1/metrics".to_string());
    let traces_timeout  = Duration::from_millis(std::env::var("OTEL_EXPORTER_OTLP_TRACES_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(5000));
    let metrics_timeout = Duration::from_millis(std::env::var("OTEL_EXPORTER_OTLP_METRICS_TIMEOUT").ok().and_then(|s| s.parse().ok()).unwrap_or(5000));

    let trace_protocol   = OtlpProtocol::Http;
    let metrics_protocol = OtlpProtocol::Http;

    // ===== exporters (iguais ao seu snippet) =====
    let span_exporter = match trace_protocol {
        OtlpProtocol::Grpc => Err(anyhow!("gRPC trace exporter não habilitado (use a feature `grpc-tonic`)")),
        OtlpProtocol::Http => {
            Ok(SpanExporter::builder()
                .with_http()
                .with_http_client(Client::new())
                .with_endpoint(traces_endpoint.clone())
                .with_protocol(Protocol::HttpBinary)
                .with_timeout(traces_timeout)
                .build())
        }
    }?;

    let metric_exporter = match metrics_protocol {
        OtlpProtocol::Grpc => Err(anyhow!("gRPC metric exporter não habilitado (use a feature `grpc-tonic`)")),
        OtlpProtocol::Http => {
            Ok(MetricExporter::builder()
                .with_http()
                .with_http_client(Client::new())
                .with_endpoint(metrics_endpoint.clone())
                .with_protocol(Protocol::HttpBinary)
                .with_timeout(metrics_timeout)
                .build())
        }
    }?;

    // ===== tracer provider (batch, Tokio) =====
    let tp = sdktrace::SdkTracerProvider::builder()
        .with_batch_exporter(span_exporter?)
        .with_resource(Resource::builder().with_attributes(vec![KeyValue::new("service.name", "credit-engine-core")]).build())
        .build();
    global::set_tracer_provider(tp.clone());

    // ===== meter provider com exportação periódica =====
    let mp = SdkMeterProvider::builder()
        .with_periodic_exporter(metric_exporter?) // cria PeriodicReader internamente
        .with_resource(Resource::builder().with_attributes(vec![KeyValue::new("service.name", "credit-engine-core")]).build())
        .build();
    global::set_meter_provider(mp.clone());

    // ===== smoke: 1 span + 1 métrica =====
    let tracer = global::tracer("otel_http_custom_client");
    tracer.in_span("test_span_http_custom_client", |_cx| {});

    let meter = global::meter("otel_http_custom_client");
    let counter = meter.u64_counter("demo_counter").build();
    counter.add(1, &[KeyValue::new("env", "dev")]);

    // flush
    mp.shutdown()?;
    Ok(())
}
