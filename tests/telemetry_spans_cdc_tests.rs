use credit_engine_core::telemetry_spans_cdc::{in_cdc_consume, span_cdc_consume, CdcConsumeAttrs};
use opentelemetry::trace::TracerProvider;
use opentelemetry::KeyValue;
use opentelemetry::Value;
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use std::panic::AssertUnwindSafe;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

fn make_attrs() -> CdcConsumeAttrs {
    CdcConsumeAttrs {
        stream: "trades".into(),
        partition: "p0".into(),
        offset_before: 1000,
        offset_after: 1042,
        records: 42,
        lag_seconds: 0.250,
    }
}

#[test]
fn span_exports_attributes_via_in_memory_exporter() {
    let exporter = InMemorySpanExporter::default();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter.clone())
        .build();
    let tracer = provider.tracer("obs1-tests");
    let layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        let attrs = make_attrs();
        let span = span_cdc_consume(&attrs);
        assert!(
            !span.is_disabled(),
            "cdc.consume span should be enabled for export"
        );
        span.in_scope(|| {});
        drop(span);
    });

    provider.force_flush().expect("force flush spans");

    let exported = exporter
        .get_finished_spans()
        .expect("spans should be captured");
    assert_eq!(1, exported.len(), "expected a single span export");
    let span = &exported[0];
    assert_eq!("cdc.consume", span.name.as_ref());

    let attributes: HashMap<String, Value> = span
        .attributes
        .iter()
        .map(|KeyValue { key, value, .. }| (key.as_str().to_string(), value.clone()))
        .collect();

    assert_eq!(Some(&Value::from("cdc_consume")), attributes.get("op"));
    assert_eq!(Some(&Value::from("trades")), attributes.get("cdc.stream"));
    assert_eq!(Some(&Value::from("p0")), attributes.get("cdc.partition"));
    assert_eq!(
        Some(&Value::from(1000_i64)),
        attributes.get("cdc.offset_before")
    );
    assert_eq!(
        Some(&Value::from(1042_i64)),
        attributes.get("cdc.offset_after")
    );
    assert_eq!(Some(&Value::from(42_i64)), attributes.get("cdc.records"));
    assert_eq!(
        Some(&Value::from(0.250_f64)),
        attributes.get("cdc.lag_seconds")
    );

    let mut attr_json = JsonMap::new();
    for (key, value) in &attributes {
        attr_json.insert(key.clone(), otel_value_to_json(value));
    }

    let mut sample = JsonMap::new();
    sample.insert("name".to_string(), JsonValue::String(span.name.to_string()));
    sample.insert("attributes".to_string(), JsonValue::Object(attr_json));
    println!(
        "telemetry_spans_cdc.sample={}",
        serde_json::to_string_pretty(&JsonValue::Object(sample)).expect("serialize sample")
    );
}

#[test]
fn wrapper_executes_closure() {
    let attrs = make_attrs();
    let result = in_cdc_consume(&attrs, || 2 + 2);
    assert_eq!(4, result);
}

fn expect_invalid(attrs: CdcConsumeAttrs, needle: &str) {
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let _ = span_cdc_consume(&attrs);
    }));
    match result {
        Ok(_) => panic!("expected panic for {:?}", needle),
        Err(payload) => {
            let message = if let Some(s) = payload.downcast_ref::<String>() {
                s.as_str()
            } else if let Some(s) = payload.downcast_ref::<&'static str>() {
                s
            } else {
                panic!("unexpected panic payload type");
            };
            assert!(
                message.contains(needle),
                "panic message `{}` did not contain `{}`",
                message,
                needle
            );
        }
    }
}

fn otel_value_to_json(value: &Value) -> JsonValue {
    match value {
        Value::Bool(v) => JsonValue::Bool(*v),
        Value::I64(v) => JsonValue::Number((*v).into()),
        Value::F64(v) => serde_json::Number::from_f64(*v)
            .map(JsonValue::Number)
            .unwrap_or(JsonValue::Null),
        Value::String(s) => JsonValue::String(s.to_string()),
        Value::Array(array) => JsonValue::String(array.to_string()),
        other => JsonValue::String(other.to_string()),
    }
}

#[test]
fn invalid_stream_rejected() {
    let mut attrs = make_attrs();
    attrs.stream = "".into();
    expect_invalid(attrs, "cdc.stream");
}

#[test]
fn invalid_partition_rejected() {
    let mut attrs = make_attrs();
    attrs.partition = "bad partition".into();
    expect_invalid(attrs, "cdc.partition");
}

#[test]
fn invalid_offset_order_rejected() {
    let mut attrs = make_attrs();
    attrs.offset_after = 999;
    expect_invalid(attrs, "cdc.offset_after");
}

#[test]
fn invalid_offset_before_lower_bound() {
    let mut attrs = make_attrs();
    attrs.offset_before = -2;
    expect_invalid(attrs, "cdc.offset_before");
}

#[test]
fn invalid_records_negative() {
    let mut attrs = make_attrs();
    attrs.records = -1;
    expect_invalid(attrs, "cdc.records");
}

#[test]
fn invalid_records_offset_gap() {
    let mut attrs = make_attrs();
    attrs.records = 50;
    attrs.offset_after = attrs.offset_before + 1;
    expect_invalid(attrs, "offset_after - offset_before");
}

#[test]
fn invalid_lag_negative() {
    let mut attrs = make_attrs();
    attrs.lag_seconds = -0.1;
    expect_invalid(attrs, "cdc.lag_seconds");
}

#[test]
fn invalid_lag_nan() {
    let mut attrs = make_attrs();
    attrs.lag_seconds = f64::NAN;
    expect_invalid(attrs, "cdc.lag_seconds");
}

#[test]
fn invalid_lag_infinite() {
    let mut attrs = make_attrs();
    attrs.lag_seconds = f64::INFINITY;
    expect_invalid(attrs, "cdc.lag_seconds");
}
