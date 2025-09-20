use anyhow::Result;
use opentelemetry::{global, KeyValue};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::{metrics::SdkMeterProvider, resource::Resource, trace::SdkTracerProvider};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub struct TelemetryHandle { pub tracer_provider: SdkTracerProvider, pub meter_provider: SdkMeterProvider }
impl TelemetryHandle { pub fn shutdown(self) -> Result<()> { self.tracer_provider.shutdown()?; self.meter_provider.shutdown()?; Ok(()) } }

pub fn init(service_name: &str) -> Result<TelemetryHandle> {
    let resource = Resource::builder().with_service_name(service_name.to_string()).build();

    let base = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());
    let traces_ep = std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT")
        .unwrap_or_else(|_| format!("{}/v1/traces", base));
    let metrics_ep = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .unwrap_or_else(|_| format!("{}/v1/metrics", base));

    // Traces
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(traces_ep)
        .build()?;
    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(span_exporter)
        .build();

    // Metrics
    let metric_exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .with_protocol(Protocol::HttpBinary)
        .with_endpoint(metrics_ep)
        .build()?;
    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_periodic_exporter(metric_exporter)
        .build();

    // Globais
    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    // Usa SdkTracer (implementa PreSampledTracer) — passa String pra lifetime 'static
    let tracer = tracer_provider.tracer(service_name.to_string());
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = Registry::default().with(filter).with(otel_layer);
    tracing::subscriber::set_global_default(subscriber).expect("set_global_default");

    Ok(TelemetryHandle { tracer_provider, meter_provider })
}

pub fn bump_test_metric() {
    let meter = global::meter("credit-engine-core");
    let counter = meter.u64_counter("ce_test_counter").build();
    counter.add(1, &[KeyValue::new("component", "telemetry_smoke")]);
}
