use std::sync::{Arc, Mutex};

use credit_engine_core::obs_policy_lints::{
    contains_pii_key, current_field_action, scrub_log, validate_metric_labels, FieldAction,
    PiiGuardLayer, PolicyError, ScrubMode,
};
use serde_json::{json, Map, Value};
use tracing::{event, Level};
use tracing_core::Event;
use tracing_subscriber::{layer::SubscriberExt, registry::Registry, Layer};

#[test]
fn scrub_log_rejects_pii_fields() {
    let log = json!({
        "msg": "swap",
        "email": "cliente@exemplo.com",
    });
    let err = scrub_log(log, ScrubMode::Reject).unwrap_err();
    match err {
        PolicyError::PiiDetected(field) => assert_eq!(field, "email"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn scrub_log_redacts_pii_fields() {
    let log = json!({
        "msg": "swap",
        "email": "cliente@exemplo.com",
        "extra": {
            "phone": "123",
        }
    });
    let sanitized = scrub_log(log, ScrubMode::Redact).expect("redaction succeeds");
    let obj = sanitized.as_object().expect("object");
    assert_eq!(
        obj.get("email"),
        Some(&Value::String("[redacted]".to_string()))
    );
    let extra = obj
        .get("extra")
        .and_then(|v| v.as_object())
        .expect("extra map");
    assert_eq!(
        extra.get("phone"),
        Some(&Value::String("[redacted]".to_string()))
    );
}

#[test]
fn contains_pii_key_detects_nested_extra() {
    let mut extra = Map::new();
    extra.insert("address".into(), Value::String("Rua ABC".into()));
    let mut root = Map::new();
    root.insert("extra".into(), Value::Object(extra));
    assert!(contains_pii_key(&root));
}

#[test]
fn validate_metric_labels_accepts_allowed_keys() {
    let labels = vec![
        ("op", "swap"),
        ("service", "ce-amm"),
        ("env", "dev"),
        ("version", "2.4.0+1a2b3c4"),
    ];
    assert!(validate_metric_labels(&labels).is_ok());
}

#[test]
fn validate_metric_labels_rejects_forbidden_keys() {
    let labels = vec![("op", "swap"), ("request_id", "abcd")];
    let err = validate_metric_labels(&labels).unwrap_err();
    match err {
        PolicyError::ForbiddenLabel(field) => assert_eq!(field, "request_id"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn validate_metric_labels_rejects_unknown_keys() {
    let labels = vec![("tenant", "ce")];
    let err = validate_metric_labels(&labels).unwrap_err();
    match err {
        PolicyError::ForbiddenLabel(field) => assert_eq!(field, "tenant"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[derive(Clone, Default)]
struct CollectLayer {
    events: Arc<Mutex<Vec<Map<String, Value>>>>,
}

impl CollectLayer {
    fn events(&self) -> Arc<Mutex<Vec<Map<String, Value>>>> {
        Arc::clone(&self.events)
    }
}

impl<S> Layer<S> for CollectLayer
where
    S: tracing::Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = TestVisitor::default();
        event.record(&mut visitor);
        self.events
            .lock()
            .expect("events lock poisoned")
            .push(visitor.values);
    }
}

#[derive(Default)]
struct TestVisitor {
    values: Map<String, Value>,
}

impl TestVisitor {
    fn record_value(&mut self, field: &tracing_core::Field, value: Value) {
        if let Some(action) = current_field_action(field.name()) {
            match action {
                FieldAction::Drop => return,
                FieldAction::Redact(replacement) => {
                    self.values.insert(field.name().to_string(), replacement);
                    return;
                }
            }
        }
        self.values.insert(field.name().to_string(), value);
    }
}

impl tracing_core::field::Visit for TestVisitor {
    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        self.record_value(field, Value::String(value.to_string()));
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        self.record_value(field, Value::Bool(value));
    }

    fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
        if let Some(number) = serde_json::Number::from_i128(value as i128) {
            self.record_value(field, Value::Number(number));
        } else {
            self.record_value(field, Value::String(value.to_string()));
        }
    }

    fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
        if let Some(number) = serde_json::Number::from_u128(value as u128) {
            self.record_value(field, Value::Number(number));
        } else {
            self.record_value(field, Value::String(value.to_string()));
        }
    }

    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn std::fmt::Debug) {
        self.record_value(field, Value::String(format!("{:?}", value)));
    }
}

#[test]
fn pii_guard_layer_drops_events_in_reject_mode() {
    let collector = CollectLayer::default();
    let events_handle = collector.events();
    let guard = PiiGuardLayer::new(ScrubMode::Reject);

    let subscriber = Registry::default().with(collector).with(guard);
    tracing::subscriber::with_default(subscriber, || {
        event!(
            Level::INFO,
            email = "cliente@exemplo.com",
            message = "blocked"
        );
    });

    assert!(events_handle.lock().unwrap().is_empty());
}

#[test]
fn pii_guard_layer_redacts_pii_fields() {
    let collector = CollectLayer::default();
    let events_handle = collector.events();
    let guard = PiiGuardLayer::new(ScrubMode::Redact);

    let subscriber = Registry::default().with(collector).with(guard);
    tracing::subscriber::with_default(subscriber, || {
        event!(Level::INFO, email = "cliente@exemplo.com", message = "ok");
    });

    let events = events_handle.lock().unwrap();
    assert_eq!(events.len(), 1);
    let event_map = &events[0];
    assert_eq!(
        event_map.get("email"),
        Some(&Value::String("[redacted]".to_string()))
    );
}
