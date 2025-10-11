use serde_json::{json, Map, Value};

/// Returns the current OpenTelemetry trace and span identifiers as hexadecimal strings.
///
/// If there is no active span, or if the span lacks OpenTelemetry context, `None` is returned.
pub fn current_trace_and_span() -> Option<(String, String)> {
    let (trace_id, span_id) = crate::telemetry_logs::try_extract_trace_ids();

    match (trace_id, span_id) {
        (Some(trace), Some(span)) => Some((trace, span)),
        _ => None,
    }
}

/// Emits a single JSON log line, automatically enriching it with `trace_id` and `span_id`
/// when an active OpenTelemetry span is available.
///
/// The function never panics: if serialization fails, a fallback JSON describing the
/// failure is emitted instead.
pub fn log_with_trace(fields: Value) {
    let mut object = match fields {
        Value::Object(map) => map,
        other => {
            let mut map = Map::new();
            map.insert("message".to_string(), other);
            map
        }
    };

    if let Some((trace_id, span_id)) = current_trace_and_span() {
        object.insert("trace_id".to_string(), Value::String(trace_id));
        object.insert("span_id".to_string(), Value::String(span_id));
    }

    let enriched = Value::Object(object);
    match serde_json::to_string(&enriched) {
        Ok(serialized) => {
            println!("{}", serialized);
        }
        Err(err) => {
            let fallback = json!({
                "logging_error": format!("failed_to_serialize_log:{err}"),
            });
            if let Ok(serialized) = serde_json::to_string(&fallback) {
                println!("{}", serialized);
            } else {
                // As a last resort, write a minimal JSON payload.
                println!("{{\"logging_error\":\"unserializable\"}}");
            }
        }
    }
}

/// Records the provided value in the histogram.
///
/// Exemplars are currently not supported by the upstream metrics SDK, so this helper only
/// records the measurement without attaching trace correlation data. Once exemplar support
/// is available, this function should be extended to propagate `trace_id` metadata.
pub fn observe_with_trace(hist: &opentelemetry::metrics::Histogram<f64>, value: f64) {
    hist.record(value, &[]);
}
