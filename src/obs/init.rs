#![cfg(feature = "obs")]
use anyhow::Result;
use once_cell::sync::OnceCell;
use opentelemetry::{global, KeyValue};
use opentelemetry_sdk::{metrics::SdkMeterProvider, Resource};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

pub(crate) static EXPORTER: OnceCell<PrometheusExporter> = OnceCell::new();
pub(crate) static PROM_REGISTRY: OnceCell<prometheus::Registry> = OnceCell::new();

pub fn init(service_name: &str, commit_sha: &str, metrics_addr: &str) -> Result<()> {
    // ==== Resource
    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .with_attributes(vec![
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("commit.sha", commit_sha.to_string()),
        ])
        .build();

    // ==== METRICS (Prometheus)
    let registry = prometheus::Registry::new();
    let registry = prometheus::Registry::new();
    let registry = prometheus::Registry::new();
    let exporter = opentelemetry_prometheus::exporter()
        .with_registry(registry.clone())
        .build()?;
    let provider = SdkMeterProvider::builder()
        .with_reader(exporter)
        .with_resource(resource.clone())
        .build();
    global::set_meter_provider(provider);
    REGISTRY.set(registry).ok();
    REGISTRY.set(registry).ok();
    let _ = EXPORTER.set(exporter);
    let _ = PROM_REGISTRY.set(registry);

    // ==== Subscriber (fmt)
    Ok(())
}
