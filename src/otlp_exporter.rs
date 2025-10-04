use std::time::Duration;

use opentelemetry_otlp as otlp_crate;

pub use otlp_crate::{ExporterBuildError, MetricExporter, Protocol, SpanExporter};

#[derive(Default, Clone, Copy)]
pub struct ExporterBuilderCompat;

pub fn new_exporter() -> ExporterBuilderCompat {
    ExporterBuilderCompat
}

impl ExporterBuilderCompat {
    pub fn http(self) -> HttpExporterBuilderCompat {
        HttpExporterBuilderCompat {
            endpoint: None,
            timeout: None,
        }
    }

    pub fn tonic(self) -> TonicExporterBuilderCompat {
        TonicExporterBuilderCompat {
            endpoint: None,
            timeout: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct HttpExporterBuilderCompat {
    endpoint: Option<String>,
    timeout: Option<Duration>,
}

impl HttpExporterBuilderCompat {
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build_span_exporter(self) -> Result<SpanExporter, ExporterBuildError> {
        let mut builder = otlp_crate::SpanExporter::builder().with_http();
        if let Some(endpoint) = self.endpoint {
            builder = builder.with_endpoint(endpoint);
        }
        if let Some(timeout) = self.timeout {
            builder = builder.with_timeout(timeout);
        }
        builder.build()
    }

    pub fn build_metrics_exporter(self) -> Result<MetricExporter, ExporterBuildError> {
        let mut builder = otlp_crate::MetricExporter::builder().with_http();
        if let Some(endpoint) = self.endpoint {
            builder = builder.with_endpoint(endpoint);
        }
        if let Some(timeout) = self.timeout {
            builder = builder.with_timeout(timeout);
        }
        builder.build()
    }
}

#[derive(Default, Clone)]
pub struct TonicExporterBuilderCompat {
    endpoint: Option<String>,
    timeout: Option<Duration>,
}

impl TonicExporterBuilderCompat {
    pub fn with_endpoint(mut self, endpoint: String) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn build_span_exporter(self) -> Result<SpanExporter, ExporterBuildError> {
        build_tonic_span_exporter(self.endpoint, self.timeout)
    }

    pub fn build_metrics_exporter(self) -> Result<MetricExporter, ExporterBuildError> {
        build_tonic_metric_exporter(self.endpoint, self.timeout)
    }
}

#[cfg(feature = "metrics-otlp-grpc")]
fn build_tonic_span_exporter(
    endpoint: Option<String>,
    timeout: Option<Duration>,
) -> Result<SpanExporter, ExporterBuildError> {
    let mut builder = otlp_crate::SpanExporter::builder().with_grpc();
    if let Some(endpoint) = endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    if let Some(timeout) = timeout {
        builder = builder.with_timeout(timeout);
    }
    builder.build()
}

#[cfg(not(feature = "metrics-otlp-grpc"))]
fn build_tonic_span_exporter(
    _endpoint: Option<String>,
    _timeout: Option<Duration>,
) -> Result<SpanExporter, ExporterBuildError> {
    Err(ExporterBuildError::InternalFailure(
        "gRPC span exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into(),
    ))
}

#[cfg(feature = "metrics-otlp-grpc")]
fn build_tonic_metric_exporter(
    endpoint: Option<String>,
    timeout: Option<Duration>,
) -> Result<MetricExporter, ExporterBuildError> {
    let mut builder = otlp_crate::MetricExporter::builder().with_grpc();
    if let Some(endpoint) = endpoint {
        builder = builder.with_endpoint(endpoint);
    }
    if let Some(timeout) = timeout {
        builder = builder.with_timeout(timeout);
    }
    builder.build()
}

#[cfg(not(feature = "metrics-otlp-grpc"))]
fn build_tonic_metric_exporter(
    _endpoint: Option<String>,
    _timeout: Option<Duration>,
) -> Result<MetricExporter, ExporterBuildError> {
    Err(ExporterBuildError::InternalFailure(
        "gRPC metrics exporter support is not enabled (enable the `metrics-otlp-grpc` feature)".into(),
    ))
}
