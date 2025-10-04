use once_cell::sync::Lazy;
use regex::Regex;
use std::fmt;
use tracing::{callsite, field, Level, Span};

const OPERATION_NAME: &str = "cdc_consume";
const SPAN_NAME: &str = "cdc.consume";

static STREAM_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-z0-9._-]{3,64}$").unwrap());
static PARTITION_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[a-zA-Z0-9._-]{1,32}$").unwrap());

#[derive(Debug, Clone)]
pub struct CdcConsumeAttrs {
    pub stream: String,
    pub partition: String,
    pub offset_before: i64,
    pub offset_after: i64,
    pub records: i64,
    pub lag_seconds: f64,
}

#[derive(Debug, Clone)]
struct CdcValidationError {
    message: String,
}

impl fmt::Display for CdcValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

fn validate_attrs(attrs: &CdcConsumeAttrs) -> Result<(), CdcValidationError> {
    if !STREAM_REGEX.is_match(attrs.stream.as_str()) {
        return Err(CdcValidationError {
            message: format!(
                "invalid cdc.consume attribute `cdc.stream`: expected ^[a-z0-9._-]{{3,64}}$, got `{}`",
                attrs.stream
            ),
        });
    }

    if !PARTITION_REGEX.is_match(attrs.partition.as_str()) {
        return Err(CdcValidationError {
            message: format!(
                "invalid cdc.consume attribute `cdc.partition`: expected ^[a-zA-Z0-9._-]{{1,32}}$, got `{}`",
                attrs.partition
            ),
        });
    }

    if attrs.offset_before < -1 {
        return Err(CdcValidationError {
            message: format!(
                "invalid cdc.consume attribute `cdc.offset_before`: expected >= -1, got {}",
                attrs.offset_before
            ),
        });
    }

    if attrs.offset_after < attrs.offset_before {
        return Err(CdcValidationError {
            message: format!(
                "invalid cdc.consume attribute `cdc.offset_after`: expected >= offset_before ({}), got {}",
                attrs.offset_before, attrs.offset_after
            ),
        });
    }

    if attrs.records < 0 {
        return Err(CdcValidationError {
            message: format!(
                "invalid cdc.consume attribute `cdc.records`: expected >= 0, got {}",
                attrs.records
            ),
        });
    }

    if !attrs.lag_seconds.is_finite() || attrs.lag_seconds < 0.0 {
        return Err(CdcValidationError {
            message: format!(
                "invalid cdc.consume attribute `cdc.lag_seconds`: expected finite >= 0, got {}",
                attrs.lag_seconds
            ),
        });
    }

    if attrs.records > 0 {
        let delta = (attrs.offset_after as i128) - (attrs.offset_before as i128);
        if delta < attrs.records as i128 {
            return Err(CdcValidationError {
                message: format!(
                    "invalid cdc.consume attribute combination: offset_after - offset_before ({} - {}) < records ({})",
                    attrs.offset_after, attrs.offset_before, attrs.records
                ),
            });
        }
    }

    Ok(())
}

pub fn span_cdc_consume(attrs: &CdcConsumeAttrs) -> Span {
    if let Err(err) = validate_attrs(attrs) {
        panic!("invalid cdc.consume attributes: {}", err);
    }

    let span = new_span(attrs);
    if span.is_disabled() {
        callsite::rebuild_interest_cache();
        return new_span(attrs);
    }

    span
}

fn new_span(attrs: &CdcConsumeAttrs) -> Span {
    tracing::span!(
        target: "obs.cdc",
        Level::INFO,
        SPAN_NAME,
        op = OPERATION_NAME,
        "cdc.stream" = field::display(&attrs.stream),
        "cdc.partition" = field::display(&attrs.partition),
        "cdc.offset_before" = attrs.offset_before,
        "cdc.offset_after" = attrs.offset_after,
        "cdc.records" = attrs.records,
        "cdc.lag_seconds" = attrs.lag_seconds
    )
}

pub fn in_cdc_consume<F, T>(attrs: &CdcConsumeAttrs, f: F) -> T
where
    F: FnOnce() -> T,
{
    let span = span_cdc_consume(attrs);
    span.in_scope(f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "invalid cdc.consume attribute `cdc.stream`")]
    fn span_cdc_consume_invalid_stream_panics() {
        let attrs = CdcConsumeAttrs {
            stream: "".into(),
            partition: "p0".into(),
            offset_before: 0,
            offset_after: 0,
            records: 0,
            lag_seconds: 0.0,
        };

        let _ = span_cdc_consume(&attrs);
    }
}
