use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry::KeyValue;
use opentelemetry_otlp::{WithExportConfig, WithHttpConfig};
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::propagation::TraceContextPropagator;
use opentelemetry_sdk::resource::Resource;
use opentelemetry_sdk::trace::{Sampler, SdkTracerProvider, SimpleSpanProcessor};
use reqwest::Client;
use thiserror::Error;
use tracing::dispatcher::{self, DefaultGuard as DispatchGuard};
use tracing::field::{Field, Visit};
use tracing::Subscriber;
use tracing_opentelemetry::{layer, OtelData};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::fmt::format::{
    FormatEvent as SubscriberFormatEvent, FormatFields as SubscriberFormatFields,
    Writer as SubscriberWriter,
};
use tracing_subscriber::fmt::{self as ts_fmt, FmtContext};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Registry;

static TRACING_ACTIVE: AtomicBool = AtomicBool::new(false);

const DEFAULT_SERVICE_NAME: &str = "credit-engine-core";
const DEFAULT_SERVICE_VERSION: &str = "0.1.0";
const DEFAULT_DEPLOY_ENV: &str = "local";
const DEFAULT_SAMPLER: &str = "parentbased_traceidratio";
const DEFAULT_SAMPLER_RATIO: f64 = 0.1;
const DEFAULT_OTLP_HTTP_URL: &str = "http://127.0.0.1:4318";
const EXPORT_TIMEOUT: Duration = Duration::from_secs(5);
const INSTRUMENTATION_SCOPE: &str = "obs4.tracing";

#[derive(Debug, Error)]
pub enum TracingInitError {
    #[error("tracing has already been initialized for this process")]
    AlreadyInitialized,
    #[error("unsupported sampler `{0}` (expected `parentbased_traceidratio`)")]
    UnsupportedSampler(String),
    #[error("invalid sampler argument `{raw}`: {source}")]
    InvalidSamplerArg {
        raw: String,
        #[source]
        source: std::num::ParseFloatError,
    },
    #[error("sampler probability must be between 0.0 and 1.0 inclusive, got {0}")]
    InvalidSamplerProbability(f64),
    #[error("failed to build OTLP HTTP exporter: {0}")]
    ExporterBuild(String),
    #[error("failed to build reqwest client: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("failed to install tracing subscriber: {0}")]
    SubscriberInstall(String),
}

#[derive(Debug, Error)]
pub enum TracingShutdownError {
    #[error("tracer provider shutdown failed: {0}")]
    Provider(#[from] OTelSdkError),
}

#[derive(Debug)]
pub struct TracingGuard {
    provider: Option<SdkTracerProvider>,
    previous_provider: Option<global::GlobalTracerProvider>,
    subscriber_guard: Option<DispatchGuard>,
    active_flag: &'static AtomicBool,
    shutdown_called: bool,
}

impl TracingGuard {
    fn new(
        provider: SdkTracerProvider,
        previous_provider: global::GlobalTracerProvider,
        subscriber_guard: DispatchGuard,
    ) -> Self {
        Self {
            provider: Some(provider),
            previous_provider: Some(previous_provider),
            subscriber_guard: Some(subscriber_guard),
            active_flag: &TRACING_ACTIVE,
            shutdown_called: false,
        }
    }

    pub fn shutdown(mut self) -> Result<(), TracingShutdownError> {
        self.shutdown_inner()
    }

    fn shutdown_inner(&mut self) -> Result<(), TracingShutdownError> {
        if self.shutdown_called {
            return Ok(());
        }

        if let Some(guard) = self.subscriber_guard.take() {
            drop(guard);
        }

        if let Some(previous) = self.previous_provider.take() {
            let _ = global::set_tracer_provider(previous);
        }

        if let Some(provider) = self.provider.take() {
            match provider.shutdown() {
                Ok(()) | Err(OTelSdkError::AlreadyShutdown) => {}
                Err(err) => return Err(TracingShutdownError::Provider(err)),
            }
        }

        self.active_flag.store(false, Ordering::Release);
        self.shutdown_called = true;
        Ok(())
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        let _ = self.shutdown_inner();
    }
}

pub fn init_tracing() -> Result<TracingGuard, TracingInitError> {
    if TRACING_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(TracingInitError::AlreadyInitialized);
    }

    let mut guard_reset = ActiveFlagReset::new(&TRACING_ACTIVE);

    let sampler_name =
        std::env::var("OTEL_TRACES_SAMPLER").unwrap_or_else(|_| DEFAULT_SAMPLER.to_string());
    if sampler_name.trim().to_ascii_lowercase() != DEFAULT_SAMPLER {
        return Err(TracingInitError::UnsupportedSampler(sampler_name));
    }

    let sampler_ratio_raw = std::env::var("OTEL_TRACES_SAMPLER_ARG").ok();
    let sampler_ratio = match sampler_ratio_raw {
        Some(raw) => {
            let parsed = raw
                .parse::<f64>()
                .map_err(|source| TracingInitError::InvalidSamplerArg { raw, source })?;
            if !(0.0..=1.0).contains(&parsed) {
                return Err(TracingInitError::InvalidSamplerProbability(parsed));
            }
            parsed
        }
        None => DEFAULT_SAMPLER_RATIO,
    };

    let sampler = Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(sampler_ratio)));

    let service_name =
        std::env::var("SERVICE_NAME").unwrap_or_else(|_| DEFAULT_SERVICE_NAME.to_string());
    let service_version =
        std::env::var("SERVICE_VERSION").unwrap_or_else(|_| DEFAULT_SERVICE_VERSION.to_string());
    let deploy_env = std::env::var("DEPLOY_ENV").unwrap_or_else(|_| DEFAULT_DEPLOY_ENV.to_string());

    let resource = Resource::builder()
        .with_service_name(service_name)
        .with_attributes([
            KeyValue::new("service.version", service_version),
            KeyValue::new("deployment.environment", deploy_env),
        ])
        .build();

    let http_base =
        std::env::var("OTLP_HTTP_URL").unwrap_or_else(|_| DEFAULT_OTLP_HTTP_URL.to_string());
    let endpoint = normalize_endpoint(&http_base);

    let http_client = Client::builder().timeout(EXPORT_TIMEOUT).build()?;
    let span_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_http_client(http_client)
        .with_endpoint(endpoint)
        .with_timeout(EXPORT_TIMEOUT)
        .build()
        .map_err(|err| TracingInitError::ExporterBuild(err.to_string()))?;

    let span_processor = SimpleSpanProcessor::new(span_exporter);

    let tracer_provider = SdkTracerProvider::builder()
        .with_span_processor(span_processor)
        .with_sampler(sampler)
        .with_resource(resource)
        .build();

    global::set_text_map_propagator(TraceContextPropagator::new());
    let previous_provider = global::set_tracer_provider(tracer_provider.clone());

    let tracer = tracer_provider.tracer(INSTRUMENTATION_SCOPE);
    let otel_layer = layer().with_tracer(tracer);

    let fmt_layer = ts_fmt::layer()
        .event_format(JsonEventFormatter)
        .with_ansi(false);

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let subscriber = Registry::default()
        .with(env_filter)
        .with(otel_layer)
        .with(fmt_layer);
    let dispatch = tracing::Dispatch::new(subscriber);

    tracing::dispatcher::set_global_default(dispatch.clone()).map_err(|err| {
        TracingInitError::SubscriberInstall(format!("global subscriber already set: {err}"))
    })?;
    let subscriber_guard = dispatcher::set_default(&dispatch);

    guard_reset.disarm();
    Ok(TracingGuard::new(
        tracer_provider,
        previous_provider,
        subscriber_guard,
    ))
}

fn normalize_endpoint(base: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    format!("{trimmed}/v1/traces")
}

#[derive(Debug)]
struct ActiveFlagReset {
    flag: &'static AtomicBool,
    armed: bool,
}

impl ActiveFlagReset {
    fn new(flag: &'static AtomicBool) -> Self {
        Self { flag, armed: true }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ActiveFlagReset {
    fn drop(&mut self) {
        if self.armed {
            self.flag.store(false, Ordering::Release);
        }
    }
}

#[derive(Default)]
struct JsonEventFormatter;

impl<S, N> SubscriberFormatEvent<S, N> for JsonEventFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> SubscriberFormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: SubscriberWriter<'_>,
        event: &tracing::Event<'_>,
    ) -> fmt::Result {
        let mut visitor = JsonVisitor::default();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let timestamp = format_timestamp(SystemTime::now());
        let (trace_id, span_id) = lookup_trace_and_span_ids(ctx);

        let mut fields = serde_json::Map::with_capacity(visitor.fields.len() + 6);
        fields.insert("timestamp".into(), serde_json::Value::String(timestamp));
        fields.insert(
            "level".into(),
            serde_json::Value::String(metadata.level().as_str().to_string()),
        );
        fields.insert(
            "target".into(),
            serde_json::Value::String(metadata.target().to_string()),
        );
        fields.insert("trace_id".into(), serde_json::Value::String(trace_id));
        fields.insert("span_id".into(), serde_json::Value::String(span_id));

        for (key, value) in visitor.fields {
            fields.insert(key, value);
        }

        let payload = serde_json::Value::Object(fields);
        let serialized = serde_json::to_string(&payload).map_err(|_| fmt::Error)?;
        writer.write_str(&serialized)?;
        writer.write_char('\n')
    }
}

fn lookup_trace_and_span_ids<S, N>(ctx: &FmtContext<'_, S, N>) -> (String, String)
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> SubscriberFormatFields<'writer> + 'static,
{
    const EMPTY_TRACE: &str = "00000000000000000000000000000000";
    const EMPTY_SPAN: &str = "0000000000000000";

    if let Some(span) = ctx.lookup_current() {
        let extensions = span.extensions();
        if let Some(otel_data) = extensions.get::<OtelData>() {
            let trace_id = otel_data
                .builder
                .trace_id
                .or_else(|| {
                    let parent_span = otel_data.parent_cx.span();
                    let parent = parent_span.span_context();
                    if parent.is_valid() {
                        Some(parent.trace_id())
                    } else {
                        None
                    }
                })
                .map(|id| format!("{id}"))
                .unwrap_or_else(|| EMPTY_TRACE.to_string());
            let span_id = otel_data
                .builder
                .span_id
                .map(|id| format!("{id}"))
                .unwrap_or_else(|| EMPTY_SPAN.to_string());
            return (trace_id, span_id);
        }
    }

    (EMPTY_TRACE.to_string(), EMPTY_SPAN.to_string())
}

#[derive(Default)]
struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl Visit for JsonVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{value:?}")),
        );
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::Number(value.into()),
        );
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        if let Some(number) = serde_json::Number::from_i128(value) {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::Number(number));
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        if let Some(number) = serde_json::Number::from_u128(value) {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::Number(number));
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(number) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), serde_json::Value::Number(number));
        } else {
            self.fields.insert(
                field.name().to_string(),
                serde_json::Value::String(value.to_string()),
            );
        }
    }
}

fn format_timestamp(now: SystemTime) -> String {
    match now.duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{:09}", duration.as_secs(), duration.subsec_nanos()),
        Err(_) => "0.000000000".to_string(),
    }
}
