use std::{collections::HashMap, time::Duration};

use opentelemetry::{metrics::MeterProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::Resource;
use thiserror::Error;
use tracing::warn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsLevel {
    Off,
    Min,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpProtocol {
    Grpc,
    Http,
}

#[derive(Debug, Clone)]
pub struct MetricsOtlpConfig {
    pub level: ObsLevel,
    pub otlp_endpoint: Option<String>,
    pub protocol: Option<OtlpProtocol>,
    pub export_interval_ms: u64,
    pub export_timeout_ms: u64,
}

pub type ResourcePairs = Vec<(&'static str, String)>;

pub struct MetricsGuard {
    provider: Option<SdkMeterProvider>,
}

impl MetricsGuard {
    fn new(provider: SdkMeterProvider) -> Self {
        Self {
            provider: Some(provider),
        }
    }

    pub fn shutdown(&mut self) {
        if let Some(provider) = self.provider.take() {
            if let Err(err) = provider.shutdown() {
                warn!(target: "telemetry::metrics", "meter provider shutdown failed: {err}");
            }
        }
    }
}

impl Drop for MetricsGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Error, Debug)]
pub enum MetricsInitError {
    #[error("invalid resource: {0}")]
    InvalidResource(String),
    #[error("missing endpoint for active metrics level")]
    MissingEndpointForActiveLevel,
    #[error("failed to build OTLP metrics exporter: {0}")]
    OtlpBuildError(String),
}

pub fn init_meter_otlp(
    cfg: MetricsOtlpConfig,
    resource_pairs: ResourcePairs,
) -> Result<(MetricsGuard, SdkMeterProvider), MetricsInitError> {
    let MetricsOtlpConfig {
        level,
        otlp_endpoint,
        protocol,
        export_interval_ms,
        export_timeout_ms,
    } = cfg;

    let resource = build_resource(resource_pairs)?;

    if matches!(level, ObsLevel::Off) {
        let provider = SdkMeterProvider::builder().with_resource(resource).build();
        let guard = MetricsGuard::new(provider.clone());
        return Ok((guard, provider));
    }

    let endpoint = otlp_endpoint
        .as_ref()
        .ok_or(MetricsInitError::MissingEndpointForActiveLevel)?
        .clone();

    let protocol = select_protocol(&endpoint, protocol);

    let exporter = build_otlp_exporter(&endpoint, protocol, export_timeout_ms)?;

    let reader = build_periodic_reader(exporter, export_interval_ms, export_timeout_ms);

    let provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    let guard = MetricsGuard::new(provider.clone());
    Ok((guard, provider))
}

pub fn named_meter(
    provider: &SdkMeterProvider,
    name: &'static str,
) -> opentelemetry::metrics::Meter {
    provider.meter(name)
}

pub fn select_protocol(endpoint: &str, explicit: Option<OtlpProtocol>) -> OtlpProtocol {
    if let Some(protocol) = explicit {
        return protocol;
    }

    let endpoint_lower = endpoint.to_ascii_lowercase();
    if endpoint_lower.contains(":4318") || endpoint_lower.contains("/v1/metrics") {
        OtlpProtocol::Http
    } else {
        OtlpProtocol::Grpc
    }
}

fn build_resource(resource_pairs: ResourcePairs) -> Result<Resource, MetricsInitError> {
    const REQUIRED_KEYS: [&str; 3] = ["service.name", "service.version", "deployment.environment"];

    if resource_pairs.len() != REQUIRED_KEYS.len() {
        return Err(MetricsInitError::InvalidResource(
            "resource must contain exactly service.name, service.version and deployment.environment".into(),
        ));
    }

    let mut values = HashMap::new();
    for (key, value) in resource_pairs {
        if !REQUIRED_KEYS.contains(&key) {
            return Err(MetricsInitError::InvalidResource(format!(
                "unexpected resource key: {key}"
            )));
        }
        if value.trim().is_empty() {
            return Err(MetricsInitError::InvalidResource(format!(
                "resource value for {key} cannot be empty",
            )));
        }
        if values.insert(key, value).is_some() {
            return Err(MetricsInitError::InvalidResource(format!(
                "duplicate resource key: {key}",
            )));
        }
    }

    let mut attributes = Vec::with_capacity(REQUIRED_KEYS.len());
    for key in REQUIRED_KEYS {
        let value = values.remove(key).ok_or_else(|| {
            MetricsInitError::InvalidResource(format!("resource missing required key: {key}"))
        })?;
        attributes.push(KeyValue::new(key, value));
    }

    Ok(Resource::builder().with_attributes(attributes).build())
}

fn build_otlp_exporter(
    endpoint: &str,
    protocol: OtlpProtocol,
    export_timeout_ms: u64,
) -> Result<opentelemetry_otlp::MetricExporter, MetricsInitError> {
    let timeout = Duration::from_millis(export_timeout_ms);
    let exporter = match protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                let mut builder = opentelemetry_otlp::MetricExporter::builder().with_tonic();
                builder = builder.with_endpoint(endpoint.to_string());
                builder = builder.with_timeout(timeout);
                builder.build()
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                return Err(MetricsInitError::OtlpBuildError(
                    "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                        .into(),
                ));
            }
        }
        OtlpProtocol::Http => {
            let mut builder = opentelemetry_otlp::MetricExporter::builder().with_http();
            builder = builder.with_endpoint(endpoint.to_string());
            builder = builder.with_timeout(timeout);
            builder.build()
        }
    };

    exporter.map_err(|err| MetricsInitError::OtlpBuildError(err.to_string()))
}

fn build_periodic_reader(
    exporter: opentelemetry_otlp::MetricExporter,
    export_interval_ms: u64,
    export_timeout_ms: u64,
) -> PeriodicReader<opentelemetry_otlp::MetricExporter> {
    let interval = Duration::from_millis(export_interval_ms);
    let timeout = Duration::from_millis(export_timeout_ms);

    PeriodicReader::builder(exporter, Tokio)
        .with_interval(interval)
        .with_timeout(timeout)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_validation_enforces_required_keys() {
        let resource = vec![
            ("service.name", "trendmarket".to_string()),
            ("service.version", "1.2.3".to_string()),
            ("deployment.environment", "prod".to_string()),
        ];
        let result = build_resource(resource);
        assert!(result.is_ok());
    }
}
