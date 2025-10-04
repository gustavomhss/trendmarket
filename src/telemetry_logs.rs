use std::fmt;

use crate::obs_policy_lints::{current_field_action, FieldAction};
use opentelemetry::trace::TraceContextExt;
use serde_json::{Map, Value};
use tracing::{
    field::{Field, Visit},
    Event, Subscriber,
};
use tracing_opentelemetry::OtelData;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::format::{DefaultFields, Writer};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, Layer};
use tracing_subscriber::registry::{LookupSpan, Registry};

#[derive(Debug, Clone)]
pub struct LogConfig {
    pub level: String,
    pub service: String,
    pub env: String,
    pub version: String,
}

#[derive(thiserror::Error, Debug)]
pub enum LogInitError {
    #[error("invalid log level: {0}")]
    InvalidLevel(String),
    #[error("formatter error: {0}")]
    FormatterError(String),
}

const BLOCKED_FIELDS: &[&str] = &["email", "cpf", "phone", "address", "name", "geo"];
const PLACEHOLDER_VALUES: &[&str] = &["TBD", "FIXME", "…", "PLACEHOLDER"];

/// Creates a JSON layer configured with the canonical log schema.
pub fn json_layer(
    cfg: &LogConfig,
) -> Result<
    Layer<Registry, DefaultFields, CanonicalJsonFormatter, fn() -> std::io::Stderr>,
    LogInitError,
> {
    validate_config(cfg)?;

    let mut sanitized = cfg.clone();
    sanitized.service = sanitized.service.trim().to_string();
    sanitized.env = sanitized.env.trim().to_ascii_lowercase();
    sanitized.version = sanitized.version.trim().to_string();
    sanitized.level = sanitized.level.trim().to_ascii_lowercase();

    let formatter = CanonicalJsonFormatter::new(sanitized);
    let layer = tracing_subscriber::fmt::layer()
        .event_format(formatter)
        .with_writer(std::io::stderr as fn() -> _)
        .with_ansi(false);

    Ok(layer)
}

/// Maps a string log level into a [`LevelFilter`].
pub fn level_filter(level: &str) -> Result<LevelFilter, LogInitError> {
    match level.to_ascii_lowercase().as_str() {
        "trace" => Ok(LevelFilter::TRACE),
        "debug" => Ok(LevelFilter::DEBUG),
        "info" => Ok(LevelFilter::INFO),
        "warn" => Ok(LevelFilter::WARN),
        "error" => Ok(LevelFilter::ERROR),
        other => Err(LogInitError::InvalidLevel(other.to_string())),
    }
}

/// Attempts to extract OpenTelemetry trace identifiers from the current span.
pub fn try_extract_trace_ids() -> (Option<String>, Option<String>) {
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    let current = tracing::Span::current();
    let otel_context = current.context();
    let span = otel_context.span();
    let span_context = span.span_context();

    if span_context.is_valid() {
        (
            Some(span_context.trace_id().to_string()),
            Some(span_context.span_id().to_string()),
        )
    } else {
        (None, None)
    }
}

fn validate_config(cfg: &LogConfig) -> Result<(), LogInitError> {
    for (name, value) in [
        ("service", cfg.service.as_str()),
        ("env", cfg.env.as_str()),
        ("version", cfg.version.as_str()),
        ("level", cfg.level.as_str()),
    ] {
        if is_placeholder(value) {
            return Err(LogInitError::FormatterError(format!(
                "{name} contains placeholder value"
            )));
        }
    }

    let env_normalized = cfg.env.trim().to_ascii_lowercase();
    if !matches!(env_normalized.as_str(), "dev" | "stg" | "prod") {
        return Err(LogInitError::FormatterError(format!(
            "env must be one of dev|stg|prod, got {0}",
            cfg.env
        )));
    }

    if level_filter(cfg.level.trim()).is_err() {
        return Err(LogInitError::FormatterError(format!(
            "level must be trace|debug|info|warn|error, got {0}",
            cfg.level
        )));
    }

    Ok(())
}

fn is_blocked_field(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if lower.starts_with("person_") {
        return true;
    }
    BLOCKED_FIELDS
        .iter()
        .any(|blocked| blocked.eq_ignore_ascii_case(name))
}

fn is_placeholder(value: &str) -> bool {
    PLACEHOLDER_VALUES
        .iter()
        .any(|placeholder| value.trim().eq_ignore_ascii_case(placeholder))
}

fn sanitize_string(value: &str) -> Option<String> {
    if is_placeholder(value) {
        None
    } else {
        Some(value.to_string())
    }
}

fn sanitize_json_value(value: Value) -> Option<Value> {
    match value {
        Value::String(s) => sanitize_string(&s).map(Value::String),
        other => Some(other),
    }
}

fn sanitize_op(op: &str) -> Option<String> {
    match op {
        "swap" | "add_liquidity" | "remove_liquidity" | "pricing" | "cdc_consume" => {
            Some(op.to_string())
        }
        _ => None,
    }
}

fn record_span_op<'a, S>(span: &tracing_subscriber::registry::SpanRef<'a, S>) -> Option<String>
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    if let Some(fields) = span
        .extensions()
        .get::<tracing_subscriber::fmt::FormattedFields<DefaultFields>>()
    {
        if let Some(op) = parse_op_from_formatted_fields(&fields.fields) {
            return Some(op);
        }
    }

    span.parent().and_then(|parent| record_span_op(&parent))
}

fn parse_op_from_formatted_fields(fields: &str) -> Option<String> {
    for segment in fields.split_whitespace() {
        if let Some(value) = segment.strip_prefix("op=") {
            let cleaned = value
                .trim_end_matches(',')
                .trim_matches(|c| c == '\\')
                .trim_matches('"');
            if let Some(valid) = sanitize_op(cleaned) {
                return Some(valid);
            }
        }
    }
    None
}

fn extract_trace_ids_from_span<S>(
    span: &tracing_subscriber::registry::SpanRef<'_, S>,
) -> (Option<String>, Option<String>)
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    if let Some(data) = span.extensions().get::<OtelData>() {
        let trace_id = data
            .builder
            .trace_id
            .or_else(|| {
                let parent_span = data.parent_cx.span();
                let span_context = parent_span.span_context();
                if span_context.is_valid() {
                    Some(span_context.trace_id())
                } else {
                    None
                }
            })
            .map(|id| id.to_string());
        let span_id = data.builder.span_id.map(|id| id.to_string());
        (trace_id, span_id)
    } else {
        (None, None)
    }
}

#[derive(Clone)]
pub struct CanonicalJsonFormatter {
    config: LogConfig,
}

impl CanonicalJsonFormatter {
    fn new(config: LogConfig) -> Self {
        Self { config }
    }

    fn timestamp(&self) -> Result<String, fmt::Error> {
        format_rfc3339(std::time::SystemTime::now()).ok_or(fmt::Error)
    }
}

impl<S, N> FormatEvent<S, N> for CanonicalJsonFormatter
where
    S: Subscriber + for<'span> LookupSpan<'span>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut visitor = EventFieldVisitor::default();
        event.record(&mut visitor);

        let mut entries: Vec<(String, Value)> = Vec::new();
        let timestamp = self.timestamp()?;
        entries.push(("ts".to_string(), Value::String(timestamp)));

        let level = event.metadata().level().as_str().to_ascii_lowercase();
        entries.push(("level".to_string(), Value::String(level)));

        let metadata = event.metadata();
        let message = visitor
            .message
            .and_then(|m| sanitize_string(&m))
            .or_else(|| sanitize_string(metadata.target()))
            .unwrap_or_else(|| "[blocked]".to_string());
        entries.push(("msg".to_string(), Value::String(message)));

        entries.push((
            "service".to_string(),
            Value::String(self.config.service.clone()),
        ));
        entries.push(("env".to_string(), Value::String(self.config.env.clone())));
        entries.push((
            "version".to_string(),
            Value::String(self.config.version.clone()),
        ));

        let current_span = ctx.lookup_current();
        let span_op = current_span.as_ref().and_then(|span| record_span_op(span));
        if let Some(op) = visitor.op.or(span_op) {
            entries.push(("op".to_string(), Value::String(op)));
        }

        let (trace_id, span_id) = current_span
            .as_ref()
            .map(|span| extract_trace_ids_from_span(span))
            .unwrap_or_else(|| try_extract_trace_ids());
        if let Some(trace_id) = trace_id {
            entries.push(("trace_id".to_string(), Value::String(trace_id)));
        }
        if let Some(span_id) = span_id {
            entries.push(("span_id".to_string(), Value::String(span_id)));
        }

        for (key, value) in visitor.extra_fields {
            entries.push((key, value));
        }

        let mut map = Map::new();
        for (key, value) in entries {
            map.insert(key, value);
        }

        let json_value = Value::Object(map);
        let serialized = serde_json::to_string(&json_value).map_err(|_| fmt::Error)?;
        writer.write_str(&serialized)?;
        writer.write_char('\n')
    }
}

fn format_rfc3339(time: std::time::SystemTime) -> Option<String> {
    use std::time::SystemTime;

    let duration = time.duration_since(SystemTime::UNIX_EPOCH).ok()?;
    let total_seconds = duration.as_secs() as i64;
    let days = total_seconds.div_euclid(86_400);
    let seconds_of_day = total_seconds.rem_euclid(86_400) as u32;

    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;

    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe as i32 + (era as i32) * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month as u32, day)
}

#[derive(Default)]
struct EventFieldVisitor {
    message: Option<String>,
    op: Option<String>,
    extra_fields: Vec<(String, Value)>,
}

impl EventFieldVisitor {
    fn record_value(&mut self, field: &Field, value: Value) {
        let name = field.name();

        if is_blocked_field(name) {
            return;
        }

        if let Some(action) = current_field_action(name) {
            match action {
                FieldAction::Drop => {
                    return;
                }
                FieldAction::Redact(replacement) => {
                    self.extra_fields.push((name.to_string(), replacement));
                    return;
                }
            }
        }

        if matches!(
            name,
            "service" | "env" | "version" | "ts" | "level" | "msg" | "trace_id" | "span_id"
        ) {
            return;
        }

        match name {
            "message" => {
                if let Value::String(s) = value {
                    self.message = Some(s);
                } else {
                    self.message = Some(value.to_string());
                }
            }
            "op" => {
                if let Value::String(s) = value {
                    if let Some(valid) = sanitize_op(&s) {
                        self.op = Some(valid);
                    }
                }
            }
            _ => {
                if let Some(clean) = sanitize_json_value(value) {
                    self.extra_fields.push((name.to_string(), clean));
                }
            }
        }
    }
}

impl Visit for EventFieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record_value(field, Value::String(format!("{:?}", value)));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record_value(field, Value::String(value.to_string()));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record_value(field, Value::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        if let Some(number) = serde_json::Number::from_i128(value as i128) {
            self.record_value(field, Value::Number(number));
        } else {
            self.record_value(field, Value::String(value.to_string()));
        }
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        if let Some(number) = serde_json::Number::from_u128(value as u128) {
            self.record_value(field, Value::Number(number));
        } else {
            self.record_value(field, Value::String(value.to_string()));
        }
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(number) = serde_json::Number::from_f64(value) {
            self.record_value(field, Value::Number(number));
        }
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record_value(field, Value::String(value.to_string()));
    }
}
