#![cfg(feature = "obs")]

use anyhow::Result;
use once_cell::sync::OnceCell;
use opentelemetry::{global, KeyValue};
use opentelemetry_prometheus as otlp_prom;
use opentelemetry_sdk::{metrics::SdkMeterProvider, Resource};
use tracing_subscriber::FmtSubscriber;

pub(crate) static REGISTRY: OnceCell<prometheus::Registry> = OnceCell::new();

pub fn init(service_name: &str, commit_sha: &str, metrics_addr: &str) -> Result<()> {
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .with_attributes(vec![
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("commit.sha", commit_sha.to_string()),
        ])
        .build();

    let registry = prometheus::Registry::new();
    let exporter = otlp_prom::exporter()
        .with_registry(registry.clone())
        .build()?;
    let provider = SdkMeterProvider::builder()
        .with_reader(exporter)
        .with_resource(resource.clone())
        .build();
    global::set_meter_provider(provider);
    let _ = REGISTRY.set(registry);

    let subscriber = FmtSubscriber::builder().with_target(false).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    start_metrics_http(metrics_addr);
    Ok(())
}

fn start_metrics_http(addr: &str) {
    let addr = addr.to_string();
    std::thread::spawn(move || {
        let server = tiny_http::Server::http(&addr).expect("bind /metrics");
        eprintln!("[obs] /metrics at http://{}/metrics", addr);
        for req in server.incoming_requests() {
            if req.url() == "/metrics" {
                let body = prometheus_text();
                let hdr =
                    tiny_http::Header::from_bytes(b"Content-Type", b"text/plain; version=0.0.4")
                        .unwrap();
                let resp = tiny_http::Response::from_string(body).with_header(hdr);
                let _ = req.respond(resp);
            } else {
                let _ = req
                    .respond(tiny_http::Response::from_string("not found").with_status_code(404));
            }
        }
    });
}

fn prometheus_text() -> String {
    use prometheus::{Encoder, TextEncoder};
    let encoder = TextEncoder::new();
    let mut buf = Vec::new();
    if let Some(registry) = REGISTRY.get() {
        let mf = registry.gather();
        let _ = encoder.encode(&mf, &mut buf);
    }
    String::from_utf8(buf).unwrap_or_default()
}
