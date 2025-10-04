#![allow(unexpected_cfgs)]

use std::collections::HashMap;
use std::fmt;
use std::panic;
use std::sync::OnceLock;
use std::time::Duration;

use self::opentelemetry_otlp::SpanExporter as OtlpSpanExporter;
use opentelemetry::global;
use opentelemetry::propagation::TextMapCompositePropagator;
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
mod opentelemetry_otlp {
    pub use ::opentelemetry_otlp::{SpanExporter, WithExportConfig};

    pub struct SpanExporterBuilderCompat;

    impl SpanExporterBuilderCompat {
        pub fn http(self) -> ::opentelemetry_otlp::HttpExporterBuilder {
            ::opentelemetry_otlp::HttpExporterBuilder::default()
        }
    }

    #[cfg(feature = "metrics-otlp-grpc")]
    impl SpanExporterBuilderCompat {
        pub fn tonic(self) -> ::opentelemetry_otlp::TonicExporterBuilder {
            ::opentelemetry_otlp::TonicExporterBuilder::default()
        }
    }

    pub fn new_exporter() -> SpanExporterBuilderCompat {
        SpanExporterBuilderCompat
    }
}

use opentelemetry_otlp::{SpanExporter as OtlpSpanExporter, WithExportConfig};
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::propagation::{BaggagePropagator, TraceContextPropagator};
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::{
    BatchConfig, BatchConfigBuilder, BatchSpanProcessor, Sampler, SdkTracerProvider,
    SpanExporter as SdkSpanExporter, Tracer,
};
use thiserror::Error;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::Registry;

mod opentelemetry_otlp {
    use std::time::Duration;

    pub use ::opentelemetry_otlp::*;

    #[derive(Clone, Copy)]
    enum ExporterKind {
        Grpc,
        Http,
    }

    pub struct SpanExporterBuilderCompat {
        kind: Option<ExporterKind>,
        endpoint: Option<String>,
        timeout: Option<Duration>,
    }

    impl SpanExporterBuilderCompat {
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

        pub fn build_span_exporter(self) -> Result<SpanExporter, ExporterBuildError> {
            match self.kind.unwrap_or(ExporterKind::Http) {
                ExporterKind::Grpc => {
                    #[cfg(feature = "metrics-otlp-grpc")]
                    {
                        let mut builder =
                            ::opentelemetry_otlp::SpanExporter::builder().with_tonic();
                        if let Some(endpoint) = self.endpoint {
                            builder = builder.with_endpoint(endpoint);
                        }
                        if let Some(timeout) = self.timeout {
                            builder = builder.with_timeout(timeout);
                        }
                        builder.build_span_exporter()
                    }
                    #[cfg(not(feature = "metrics-otlp-grpc"))]
                    {
                        Err(ExporterBuildError::InternalFailure(
                            "gRPC trace exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                                .into(),
                        ))
                    }
                }
                ExporterKind::Http => {
                    let mut builder = ::opentelemetry_otlp::SpanExporter::builder().with_http();
                    if let Some(endpoint) = self.endpoint {
                        builder = builder.with_endpoint(endpoint);
                    }
                    if let Some(timeout) = self.timeout {
                        builder = builder.with_timeout(timeout);
                    }
                    builder.build_span_exporter()
                }
            }
        }
    }

    pub fn new_exporter() -> SpanExporterBuilderCompat {
        SpanExporterBuilderCompat {
            kind: None,
            endpoint: None,
            timeout: None,
        }
    }
}

const REQUIRED_RESOURCE_KEYS: [&str; 3] =
    ["service.name", "service.version", "deployment.environment"];

const TRACER_NAME: &str = "obs1.telemetry";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObsLevel {
    Off,
    Min,
    Full,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OtlpProtocol {
    Grpc,
    Http,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceConfig {
    pub level: ObsLevel,
    pub otlp_endpoint: Option<String>,
    pub protocol: Option<OtlpProtocol>,
    pub export_timeout_ms: u64,
    pub max_queue_size: usize,
    pub scheduled_delay_ms: u64,
    pub max_export_batch_size: usize,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            level: ObsLevel::Off,
            otlp_endpoint: None,
            protocol: None,
            export_timeout_ms: 10_000,
            max_queue_size: 2_048,
            scheduled_delay_ms: 5_000,
            max_export_batch_size: 512,
        }
    }
}

pub type ResourcePairs = Vec<(&'static str, String)>;

#[derive(Debug)]
pub struct TraceGuard {
    provider: Option<SdkTracerProvider>,
    shutdown_called: bool,
}

impl TraceGuard {
    fn new(provider: SdkTracerProvider) -> Self {
        Self {
            provider: Some(provider),
            shutdown_called: false,
        }
    }

    pub fn shutdown(&mut self) {
        if self.shutdown_called {
            return;
        }

        if let Some(provider) = self.provider.as_ref() {
            match provider.shutdown() {
                Ok(()) => {}
                Err(OTelSdkError::AlreadyShutdown) => {}
                Err(err) => {
                    eprintln!(
                        "telemetry_trace: tracer provider shutdown reported error: {}",
                        err
                    );
                }
            }
        }

        self.shutdown_called = true;
        self.provider = None;
    }
}

impl Drop for TraceGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[derive(Error, Debug, Clone)]
pub enum TraceInitError {
    #[error("invalid resource configuration: {0}")]
    InvalidResource(String),
    #[error("missing OTLP endpoint for active sampling level")]
    MissingEndpointForActiveLevel,
    #[error("failed to build OTLP exporter: {0}")]
    OtlpBuildError(String),
    #[error("failed to register global propagator: {0}")]
    GlobalPropagatorError(String),
}

pub fn init_tracing(
    cfg: TraceConfig,
    resource: ResourcePairs,
) -> Result<(TraceGuard, OpenTelemetryLayer<Registry, Tracer>), TraceInitError> {
    let exporter = match cfg.level {
        ObsLevel::Off => None,
        ObsLevel::Min | ObsLevel::Full => {
            let endpoint = cfg
                .otlp_endpoint
                .as_ref()
                .ok_or(TraceInitError::MissingEndpointForActiveLevel)?;
            let protocol = select_protocol(endpoint, cfg.protocol.clone());
            Some(build_otlp_exporter(&cfg, endpoint, protocol)?)
        }
    };

    init_tracing_internal(cfg, resource, exporter)
}

pub fn init_tracing_with_exporter<E>(
    cfg: TraceConfig,
    resource: ResourcePairs,
    exporter: E,
) -> Result<(TraceGuard, OpenTelemetryLayer<Registry, Tracer>), TraceInitError>
where
    E: SdkSpanExporter + Send + Sync + fmt::Debug + 'static,
{
    init_tracing_internal(cfg, resource, Some(exporter))
}

pub fn select_protocol(endpoint: &str, explicit: Option<OtlpProtocol>) -> OtlpProtocol {
    if let Some(protocol) = explicit {
        return protocol;
    }

    let normalized = endpoint.trim();
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.ends_with("/v1/traces") || normalized.contains(":4318") {
        OtlpProtocol::Http
    } else {
        OtlpProtocol::Grpc
    }
}

pub fn sampler_for_level(level: ObsLevel) -> Sampler {
    match level {
        ObsLevel::Off => Sampler::AlwaysOff,
        ObsLevel::Min => Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(0.01))),
        ObsLevel::Full => Sampler::ParentBased(Box::new(Sampler::AlwaysOn)),
    }
}

fn init_tracing_internal<E>(
    cfg: TraceConfig,
    resource_pairs: ResourcePairs,
    exporter: Option<E>,
) -> Result<(TraceGuard, OpenTelemetryLayer<Registry, Tracer>), TraceInitError>
where
    E: SdkSpanExporter + Send + Sync + fmt::Debug + 'static,
{
    if !matches!(cfg.level, ObsLevel::Off) && exporter.is_none() {
        return Err(TraceInitError::MissingEndpointForActiveLevel);
    }

    let resource = build_resource(resource_pairs)?;
    install_global_propagator()?;

    let mut provider_builder = SdkTracerProvider::builder()
        .with_sampler(sampler_for_level(cfg.level))
        .with_resource(resource);

    if let Some(exporter) = exporter {
        let batch_config = build_batch_config(&cfg);
        let processor = BatchSpanProcessor::builder(exporter)
            .with_batch_config(batch_config)
            .build();
        provider_builder = provider_builder.with_span_processor(processor);
    }

    let provider = provider_builder.build();
    let tracer = provider.tracer(TRACER_NAME);
    let guard = TraceGuard::new(provider);
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Ok((guard, layer))
}

fn build_otlp_exporter(
    cfg: &TraceConfig,
    endpoint: &str,
    protocol: OtlpProtocol,
) -> Result<OtlpSpanExporter, TraceInitError> {
    let timeout = Duration::from_millis(cfg.export_timeout_ms);
    match protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(endpoint.to_string())
                    .with_endpoint(endpoint.to_owned())
                    .with_timeout(timeout)
                    .build_span_exporter()
                    .map_err(|err| TraceInitError::OtlpBuildError(err.to_string()))
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                Err(TraceInitError::OtlpBuildError(
                    "gRPC trace exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into(),
                    "gRPC trace exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                        .into(),
                ))
            }
        }
        OtlpProtocol::Http => opentelemetry_otlp::new_exporter()
            .http()
            .with_endpoint(endpoint.to_string())
            .with_endpoint(endpoint.to_owned())
            .with_timeout(timeout)
            .build_span_exporter()
            .map_err(|err| TraceInitError::OtlpBuildError(err.to_string())),
    }
}

fn build_batch_config(cfg: &TraceConfig) -> BatchConfig {
    BatchConfigBuilder::default()
        .with_max_queue_size(cfg.max_queue_size)
        .with_max_export_batch_size(cfg.max_export_batch_size)
        .with_scheduled_delay(Duration::from_millis(cfg.scheduled_delay_ms))
        .build()
}

fn build_resource(pairs: ResourcePairs) -> Result<Resource, TraceInitError> {
    if pairs.len() != REQUIRED_RESOURCE_KEYS.len() {
        return Err(TraceInitError::InvalidResource(format!(
            "expected {} resource entries, found {}",
            REQUIRED_RESOURCE_KEYS.len(),
            pairs.len()
        )));
    }

    let mut values: HashMap<&'static str, String> = HashMap::new();
    for (key, value) in pairs {
        if !REQUIRED_RESOURCE_KEYS.contains(&key) {
            return Err(TraceInitError::InvalidResource(format!(
                "unexpected resource key '{key}'"
            )));
        }
        if value.trim().is_empty() {
            return Err(TraceInitError::InvalidResource(format!(
                "resource value for '{key}' cannot be empty"
            )));
        }
        if values.insert(key, value).is_some() {
            return Err(TraceInitError::InvalidResource(format!(
                "duplicate resource key '{key}'"
            )));
        }
    }

    let attributes = REQUIRED_RESOURCE_KEYS
        .iter()
        .map(|key| {
            let value = values.remove(key).ok_or_else(|| {
                TraceInitError::InvalidResource(format!("resource key '{key}' is missing"))
            })?;
            Ok(KeyValue::new(*key, value))
        })
        .collect::<Result<Vec<_>, TraceInitError>>()?;

    let resource = Resource::builder_empty()
        .with_attributes(attributes)
        .build();

    Ok(resource)
}

fn install_global_propagator() -> Result<(), TraceInitError> {
    static INSTALL_RESULT: OnceLock<Result<(), TraceInitError>> = OnceLock::new();
    INSTALL_RESULT
        .get_or_init(|| {
            panic::catch_unwind(|| {
                let propagator = TextMapCompositePropagator::new(vec![
                    Box::new(TraceContextPropagator::new())
                        as Box<dyn opentelemetry::propagation::TextMapPropagator + Send + Sync>,
                    Box::new(BaggagePropagator::new())
                        as Box<dyn opentelemetry::propagation::TextMapPropagator + Send + Sync>,
                ]);
                global::set_text_map_propagator(propagator);
            })
            .map_err(|_| {
                TraceInitError::GlobalPropagatorError(
                    "set_text_map_propagator panicked".to_string(),
                )
            })
        })
        .clone()
}
