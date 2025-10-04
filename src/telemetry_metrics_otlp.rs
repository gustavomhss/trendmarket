#![allow(unexpected_cfgs)]

use std::{collections::HashMap, time::Duration};

use crate::otlp_exporter as opentelemetry_otlp;
use ::opentelemetry_otlp::WithExportConfig;
use opentelemetry::{metrics::MeterProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider, Temporality};
use opentelemetry_otlp::{MetricExporter, WithExportConfig};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::Resource;
use ::opentelemetry_otlp::WithExportConfig;
use thiserror::Error;
use tracing::warn;

use ::opentelemetry_otlp::WithExportConfig as _;

mod opentelemetry_otlp {
    use std::time::Duration;

    use ::opentelemetry_otlp::{ExportConfig, WithExportConfig};
    pub use ::opentelemetry_otlp::*;

    #[derive(Clone, Copy)]
    enum ExporterKind {
        Grpc,
        Http,
    }

    pub struct MetricExporterBuilderCompat {
        kind: Option<ExporterKind>,
        endpoint: Option<String>,
        timeout: Option<Duration>,
    }

    fn apply_export_config<B>(
        builder: B,
        endpoint: Option<String>,
        timeout: Option<Duration>,
    ) -> B
    where
        B: WithExportConfig,
    {
        if endpoint.is_none() && timeout.is_none() {
            builder
        } else {
            let mut export_config = ExportConfig::default();
            export_config.endpoint = endpoint;
            export_config.timeout = timeout;
            builder.with_export_config(export_config)
        }
    }

    impl MetricExporterBuilderCompat {
        pub fn tonic(mut self) -> Self {
            self.kind = Some(ExporterKind::Grpc);
            self
        }

        pub fn http(mut self) -> Self {
            self.kind = Some(ExporterKind::Http);
            self
        }

        pub fn with_endpoint(mut self, endpoint: String) -> Self {
            self.endpoint = Some(endpoint);
            self
        }

        pub fn with_timeout(mut self, timeout: Duration) -> Self {
            self.timeout = Some(timeout);
            self
        }

        pub fn build_metric_exporter(self) -> Result<MetricExporter, ExporterBuildError> {
            let MetricExporterBuilderCompat {
                kind,
                endpoint,
                timeout,
            } = self;

            match (kind.unwrap_or(ExporterKind::Http), endpoint, timeout) {
                (ExporterKind::Grpc, endpoint, timeout) => {
                    let _ = endpoint;
                    let _ = timeout;
                    Err(ExporterBuildError::InternalFailure(
                        "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into(),
                    ))
                }
                (ExporterKind::Http, endpoint, timeout) => {
                    let builder = ::opentelemetry_otlp::MetricExporter::builder().with_http();
                    let builder = apply_export_config(builder, endpoint, timeout);
use self::opentelemetry_otlp::WithExportConfig;

mod opentelemetry_otlp {
    pub use ::opentelemetry_otlp::*;
    pub use ::opentelemetry_otlp::WithExportConfig;
}
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::Resource;
use thiserror::Error;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::Resource;
use thiserror::Error;

        pub fn build_metric_exporter(self) -> Result<MetricExporter, ExporterBuildError> {
            match self.kind.unwrap_or(ExporterKind::Http) {
                ExporterKind::Grpc => {
                    #[cfg(all(
                        feature = "metrics-otlp-grpc",
                        feature = "opentelemetry-otlp/grpc-tonic"
                    ))]
                    {
                        let mut builder =
                            ::opentelemetry_otlp::MetricExporter::builder().with_tonic();
                        if let Some(endpoint) = self.endpoint {
                            builder = builder.with_endpoint(endpoint);
                        }
                        if let Some(timeout) = self.timeout {
                            builder = builder.with_timeout(timeout);
                        }
                        builder.build()
                    }
                    #[cfg(not(all(
                        feature = "metrics-otlp-grpc",
                        feature = "opentelemetry-otlp/grpc-tonic"
                    )))]
                    {
                        Err(ExporterBuildError::InternalFailure(
                            "gRPC metrics exporter support is not enabled".into(),
                        ))
                    }
                }
                ExporterKind::Grpc => Err(ExporterBuildError::InternalFailure(
                    "gRPC metrics exporter support is not enabled".into(),
                )),
                ExporterKind::Http => {
                    let mut builder = ::opentelemetry_otlp::MetricExporter::builder().with_http();
                    if let Some(endpoint) = self.endpoint {
                        builder = builder.with_endpoint(endpoint);
                    }
                    if let Some(timeout) = self.timeout {
                        builder = builder.with_timeout(timeout);
                    }
                    builder.build()
                }
            }
        }
    }
use ::opentelemetry_otlp::WithExportConfig;
use tracing::warn;

use self::opentelemetry_otlp::WithExportConfig;

use ::opentelemetry_otlp::WithExportConfig;

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

    let reader = build_periodic_reader(exporter, export_interval_ms);

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
) -> Result<MetricExporter, MetricsInitError> {
    let timeout = Duration::from_millis(export_timeout_ms);

    match protocol {
        OtlpProtocol::Grpc => {
            #[cfg(all(
                feature = "metrics-otlp-grpc",
                feature = "opentelemetry-otlp/grpc-tonic"
            ))]
            {
                let mut builder = opentelemetry_otlp::MetricExporter::builder().with_tonic();
                builder = builder.with_endpoint(endpoint.to_string());
                builder = builder.with_timeout(timeout);
                builder
                    .build()
                    .map_err(|err| MetricsInitError::OtlpBuildError(err.to_string()))
            }
            #[cfg(not(all(
                feature = "metrics-otlp-grpc",
                feature = "opentelemetry-otlp/grpc-tonic"
            )))]
            {
                Err(MetricsInitError::OtlpBuildError(
                    "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature with `opentelemetry-otlp` gRPC support)"
                        .into(),
                ))
            }
        }
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                opentelemetry_otlp::TonicExporterBuilder::default()
                    .with_endpoint(endpoint.to_string())
                    .with_timeout(timeout)
                    .build_metrics_exporter(Temporality::Cumulative)
                MetricExporter::builder()
                    .with_tonic()
                    .with_endpoint(endpoint.to_string())
                    .with_timeout(timeout)
                    .build()
                    .map_err(|err| MetricsInitError::OtlpBuildError(err.to_string()))
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                Err(MetricsInitError::OtlpBuildError(
                    "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into(),
                ))
            }
        }
        OtlpProtocol::Http => MetricExporter::builder()
                let mut builder = MetricExporter::builder().with_tonic();
                builder = builder.with_endpoint(endpoint.to_string());
                builder = builder.with_timeout(timeout);
                builder
                    .build()
        OtlpProtocol::Grpc => Err(MetricsInitError::OtlpBuildError(
            if cfg!(feature = "metrics-otlp-grpc") {
                "gRPC metrics exporter support is not available in this build".into()
            } else {
                "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into()
            },
            "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                .into(),
        )),
        OtlpProtocol::Http => {
            let mut export_config = ::opentelemetry_otlp::ExportConfig::default();
            export_config.endpoint = Some(endpoint.to_string());
            export_config.timeout = Some(timeout);
            ::opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .with_export_config(export_config)
                .build()
                .map_err(|err| MetricsInitError::OtlpBuildError(err.to_string()))
        }
        OtlpProtocol::Http => opentelemetry_otlp::MetricExporter::builder()
            .with_http()
        OtlpProtocol::Grpc => {
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint.to_string())
                    .with_timeout(timeout)
                    .build_metrics_exporter()
                    .map_err(|err| MetricsInitError::OtlpBuildError(err.to_string()))
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                Err(MetricsInitError::OtlpBuildError(
                    "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                        .into(),
                ))
            }
        }
        OtlpProtocol::Http => opentelemetry_otlp::HttpExporterBuilder::default()
            .with_endpoint(endpoint.to_string())
            .with_timeout(timeout)
            .build_metrics_exporter(Temporality::Cumulative)
        OtlpProtocol::Http => {
            let mut builder = MetricExporter::builder().with_http();
            builder = builder.with_endpoint(endpoint.to_string());
            builder = builder.with_timeout(timeout);
            builder
                .build()
                .map_err(|err| MetricsInitError::OtlpBuildError(err.to_string()))
        }
                    "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into(),
                ))
            }
        }
        OtlpProtocol::Http => opentelemetry_otlp::new_exporter()
            .http()
            .with_endpoint(endpoint.to_string())
            .with_timeout(timeout)
            .build_metrics_exporter()
            .map_err(|err| MetricsInitError::OtlpBuildError(err.to_string())),
    }
}

fn build_periodic_reader(
    exporter: MetricExporter,
    export_interval_ms: u64,
) -> PeriodicReader<MetricExporter> {
    let interval = Duration::from_millis(export_interval_ms);

    PeriodicReader::builder(exporter)
        .with_interval(interval)
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
