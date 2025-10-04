use anyhow::{bail, Result};
use anyhow::{anyhow, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[cfg(feature = "obs")]
use metrics_exporter_prometheus::PrometheusBuilder;
#[cfg(feature = "obs")]
use std::net::SocketAddr;

use opentelemetry::{
    global,
    metrics::{Histogram, Meter, MeterProvider},
    trace::TracerProvider as _,
    KeyValue,
};
use opentelemetry_otlp::{MetricExporter, SpanExporter};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    propagation::TraceContextPropagator,
    resource::Resource,
    trace::SdkTracerProvider,
};
use tracing::Level;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

use crate::telemetry_metrics_otlp::{build_metrics_exporter, OtlpProtocol};

pub struct Telemetry {
    pub tracer_provider: SdkTracerProvider,
    pub meter_provider: SdkMeterProvider,
    pub meter: Meter,
    pub swap_latency_ms: Histogram<f64>,
    pub invariant_error_rel: Histogram<f64>,
    shutdown_called: AtomicBool,
}

impl Telemetry {
    pub fn shutdown(&self) {
        if self.shutdown_called.swap(true, Ordering::AcqRel) {
            return;
        }

        let _ = self.meter_provider.force_flush();
        let _ = self.tracer_provider.shutdown();
    }
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn init(service_name: &str) -> Result<Telemetry> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4318".to_string());
    let traces_endpoint =
        std::env::var("OTEL_EXPORTER_OTLP_TRACES_ENDPOINT").unwrap_or_else(|_| endpoint.clone());
    let metrics_endpoint = std::env::var("OTEL_EXPORTER_OTLP_METRICS_ENDPOINT")
        .unwrap_or_else(|_| endpoint.clone());

    let default_timeout =
        env_timeout("OTEL_EXPORTER_OTLP_TIMEOUT").unwrap_or_else(|| Duration::from_secs(10));
    let traces_timeout = env_timeout("OTEL_EXPORTER_OTLP_TRACES_TIMEOUT").unwrap_or(default_timeout);
    let metrics_timeout = env_timeout("OTEL_EXPORTER_OTLP_METRICS_TIMEOUT").unwrap_or(default_timeout);

    let commit = std::env::var("CE_COMMIT_SHA").unwrap_or_else(|_| "unknown".into());

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", service_name.to_string()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("git.commit.sha", commit),
        ])
        .build();

    // ---- Traces ----
    let trace_protocol = select_otlp_protocol("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL");
    let span_exporter = match trace_protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                SpanExporter::builder()
                    .with_grpc()
                    .with_endpoint(traces_endpoint.clone())
                    .with_timeout(traces_timeout)
                    .build()?
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                bail!(
                    "gRPC trace exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                );
            }
        }
        OtlpProtocol::Http => SpanExporter::builder()
            .with_http()
            .with_endpoint(traces_endpoint.clone())
            .with_timeout(traces_timeout)
            .build()?,
    };
    let trace_protocol = select_otlp_protocol("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL");
    let span_exporter = build_span_exporter(&traces_endpoint, trace_protocol, traces_timeout)?;

    let tracer_provider = SdkTracerProvider::builder()
        .with_resource(resource.clone())
        .with_batch_exporter(span_exporter)
        .build();

    let tracer = tracer_provider.tracer("ce_core");

    // ---- Metrics ----
    let metrics_protocol = select_otlp_protocol("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL");
    let metric_exporter = match metrics_protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                MetricExporter::builder()
                    .with_grpc()
                    .with_endpoint(metrics_endpoint.clone())
                    .with_timeout(metrics_timeout)
                    .build()?
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                bail!(
                    "gRPC metric exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                );
            }
        }
        OtlpProtocol::Http => MetricExporter::builder()
            .with_http()
            .with_endpoint(metrics_endpoint.clone())
            .with_timeout(metrics_timeout)
            .build()?,
    };
    let metrics_protocol = select_otlp_protocol("OTEL_EXPORTER_OTLP_METRICS_PROTOCOL");
    let metric_exporter =
        build_metrics_exporter(&metrics_endpoint, metrics_protocol, metrics_timeout)
            .map_err(|err| anyhow!(err.to_string()))?;

    let reader = PeriodicReader::builder(metric_exporter)
        .with_interval(Duration::from_secs(10))
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .build();

    global::set_text_map_propagator(TraceContextPropagator::new());
    global::set_tracer_provider(tracer_provider.clone());
    global::set_meter_provider(meter_provider.clone());

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(false);
    let subscriber = Registry::default()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt_layer)
        .with(otel_layer);
    let _ = tracing::subscriber::set_global_default(subscriber);

    let meter = meter_provider.meter("ce_core");
    let swap_latency_ms = meter
        .f64_histogram("swap_latency_ms")
        .with_unit("ms")
        .with_description("Latency of swap operations in ms")
        .build();
    let invariant_error_rel = meter
        .f64_histogram("invariant_error_rel")
        .with_unit("1")
        .with_description("Relative invariant error |Δk/k| per operation")
        .build();

    Ok(Telemetry {
        tracer_provider,
        meter_provider,
        meter,
        swap_latency_ms,
        invariant_error_rel,
        shutdown_called: AtomicBool::new(false),
    })
}

fn build_span_exporter(
    endpoint: &str,
    protocol: OtlpProtocol,
    timeout: Duration,
) -> Result<SpanExporter> {
    let mut builder = SpanExporter::builder();

    #[allow(unused_mut)]
    let mut builder = match protocol {
        OtlpProtocol::Grpc => {
            #[cfg(feature = "metrics-otlp-grpc")]
            {
                builder.with_grpc()
            }
            #[cfg(not(feature = "metrics-otlp-grpc"))]
            {
                return Err(anyhow!(
                    "gRPC trace exporter support is not enabled (enable the `metrics-otlp-grpc` feature)"
                ));
            }
        }
        OtlpProtocol::Http => builder.with_http(),
    };

    builder = builder.with_endpoint(endpoint.to_string());
    builder = builder.with_timeout(timeout);

    builder.build().map_err(|err| anyhow!(err.to_string()))
}

fn select_otlp_protocol(specific_env_var: &str) -> OtlpProtocol {
    env_otlp_protocol(specific_env_var)
        .or_else(|| env_otlp_protocol("OTEL_EXPORTER_OTLP_PROTOCOL"))
        .unwrap_or(OtlpProtocol::Http)
}

fn env_otlp_protocol(var: &str) -> Option<OtlpProtocol> {
    std::env::var(var)
        .ok()
        .and_then(|value| parse_otlp_protocol(&value))
}

fn parse_otlp_protocol(raw: &str) -> Option<OtlpProtocol> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "grpc" => Some(OtlpProtocol::Grpc),
        "http" | "http/proto" | "http/protobuf" | "http-proto" | "http-protobuf" | "http_json"
        | "http/json" => Some(OtlpProtocol::Http),
        _ => None,
    }
}

fn env_timeout(var: &str) -> Option<Duration> {
    std::env::var(var).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            trimmed.parse::<u64>().ok().map(Duration::from_millis)
        }
    })
}

/// Cria um `Span` INFO com nome **estático** (exigência do tracing) e
/// coloca o nome dinâmico em `span_name`. Inclui `git_commit_sha`.
pub fn make_info_span(name: &str, op_id: u32, component: &str) -> tracing::Span {
    let commit = std::env::var("CE_COMMIT_SHA").unwrap_or_else(|_| "unknown".into());
    tracing::span!(
        target: "ce_core",
        Level::INFO,
        "op",
        git_commit_sha = %commit,
        span_name = %name,
        op_id = op_id,
        component = component
    )
}

#[cfg(feature = "obs")]
pub fn start_prometheus(addr: &str) -> anyhow::Result<()> {
    let sock: SocketAddr = addr.parse()?;
    PrometheusBuilder::new()
        .with_http_listener(sock)
        .install()?;
    Ok(())
}

#[cfg(feature = "obs")]
pub fn inc_swap(pair: &str) {
    metrics::counter!("amm_swaps_total", "pair" => pair).increment(1);
}

#[cfg(feature = "obs")]
pub fn inc_liquidity(op: &str) {
    metrics::counter!("amm_liquidity_ops_total", "op" => op).increment(1);
}

#[cfg(feature = "obs")]
pub fn inc_error(code: &str) {
    metrics::counter!("amm_error_total", "code" => code).increment(1);
}

#[cfg(feature = "obs")]
pub fn observe_swap_latency_ms(v: f64) {
    metrics::histogram!("amm_swap_latency_ms").record(v);
}

#[cfg(test)]
mod tests {
    use super::*;
    use opentelemetry::{
        global,
        propagation::{Extractor, Injector},
        trace::{TraceContextExt, Tracer},
        Context,
    };
    use std::collections::HashMap;

    #[derive(Default)]
    struct TestCarrier(HashMap<String, String>);

    impl Injector for TestCarrier {
        fn set(&mut self, key: &str, value: String) {
            self.0.insert(key.to_string(), value);
        }
    }

    impl Extractor for TestCarrier {
        fn get(&self, key: &str) -> Option<&str> {
            self.0.get(key).map(|value| value.as_str())
        }

        fn keys(&self) -> Vec<&str> {
            self.0.keys().map(|key| key.as_str()).collect()
        }
    }

    #[test]
    fn trace_context_propagator_injects_and_extracts_after_init() {
        let telemetry = init("test-service").expect("telemetry init");

        let tracer = global::tracer("test");
        let span = tracer.start("parent");
        let cx = Context::current_with_span(span);

        let mut carrier = TestCarrier::default();
        global::get_text_map_propagator(|propagator| {
            propagator.inject_context(&cx, &mut carrier);
        });

        let extracted_cx =
            global::get_text_map_propagator(|propagator| propagator.extract(&carrier));

        let span_context = cx.span().span_context().clone();
        let extracted_span_context = extracted_cx.span().span_context().clone();

        assert!(span_context.is_valid());
        assert_eq!(span_context.trace_id(), extracted_span_context.trace_id());
        assert_eq!(span_context.span_id(), extracted_span_context.span_id());

        cx.span().end();
        telemetry.shutdown();
    }
}
