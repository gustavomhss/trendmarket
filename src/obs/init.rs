use anyhow::Result;
use once_cell::sync::OnceCell;
use opentelemetry::{global, KeyValue};
use opentelemetry_prometheus::PrometheusExporter;
use opentelemetry_sdk::{Resource, metrics::SdkMeterProvider};
use opentelemetry_otlp::WithExportConfig;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};
use tracing_opentelemetry::OpenTelemetryLayer;

static EXPORTER: OnceCell<PrometheusExporter> = OnceCell::new();

pub fn init(service_name: &str, commit_sha: &str, metrics_addr: &str) -> Result<()> {
    // ==== Resource comum
    let resource = Resource::new(vec![
        KeyValue::new("service.name", service_name.to_string()),
        KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
        KeyValue::new("commit.sha", commit_sha.to_string()),
    ]);

    // ==== METRICS (Prometheus)
    let exporter = opentelemetry_prometheus::exporter()
        .with_resource(resource.clone())
        .build()?;
    let provider = SdkMeterProvider::builder()
        .with_reader(exporter.clone())
        .with_resource(resource.clone())
        .build();
    global::set_meter_provider(provider);
    EXPORTER.set(exporter).ok();

    // ==== TRACES (OTLP → Jaeger)
    // Jaeger all-in-one expõe OTLP gRPC em :4317 por padrão
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic().with_endpoint("http://127.0.0.1:4317"))
        .with_trace_config(opentelemetry_sdk::trace::config().with_resource(resource.clone()))
        .install_simple()?;
    let otel_layer = OpenTelemetryLayer::new(tracer);

    // ==== Subscriber (fmt + otel)
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false).with_ansi(true);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into());
    let subscriber = Registry::default().with(env_filter).with(fmt_layer).with(otel_layer);
    tracing::subscriber::set_global_default(subscriber)?;

    // ==== HTTP /metrics
    start_metrics_http(metrics_addr);
    Ok(())
}

fn start_metrics_http(addr: &str) {
    let addr = addr.to_string();
    std::thread::spawn(move || {
        let server = tiny_http::Server::http(&addr).expect("bind /metrics");
        eprintln!("[obs] /metrics at http://{}/metrics", addr);
        for mut req in server.incoming_requests() {
            if req.url() == "/metrics" {
                let body = prometheus_text();
                let hdr = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/plain; version=0.0.4"[..]).unwrap();
                let resp = tiny_http::Response::from_string(body).with_header(hdr);
                let _ = req.respond(resp);
            } else {
                let _ = req.respond(tiny_http::Response::from_string("not found").with_status_code(404));
            }
        }
    });
}

fn prometheus_text() -> String {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    let registry = EXPORTER.get().expect("prom exporter").registry().clone();
    let mf = registry.gather();
    let _ = encoder.encode(&mf, &mut buf);
    String::from_utf8(buf).unwrap_or_default()
}
