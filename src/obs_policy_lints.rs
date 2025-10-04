use std::cell::RefCell;
use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};
use tracing::Subscriber;
use tracing_core::Event;
use tracing_subscriber::{layer::Context, Layer};

pub const ALLOWED_LABELS: &[&str] = &["op", "service", "env", "version"];
pub const FORBIDDEN_LABEL_KEY_REGEXES: &str =
    "(?i)(user_id|account_id|request_id|session_id|.*_uuid|.*_hash)";
pub const PII_FIELD_REGEXES: &str = "(?i)^(email|cpf|phone|address|name|geo|person_.*)$";

static ALLOWED_LABEL_SET: Lazy<HashMap<&'static str, ()>> =
    Lazy::new(|| ALLOWED_LABELS.iter().copied().map(|k| (k, ())).collect());

static FORBIDDEN_LABEL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(FORBIDDEN_LABEL_KEY_REGEXES).unwrap());
static PII_FIELD_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(PII_FIELD_REGEXES).unwrap());

#[derive(thiserror::Error, Debug)]
pub enum PolicyError {
    #[error("forbidden label key: {0}")]
    ForbiddenLabel(String),
    #[error("pii detected: {0}")]
    PiiDetected(String),
    #[error("unexpected json shape: {0}")]
    JsonShape(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrubMode {
    Reject,
    Redact,
}

#[derive(Clone, Debug)]
pub enum FieldAction {
    Drop,
    Redact(Value),
}

#[derive(Default, Debug)]
struct GuardState {
    actions: HashMap<String, FieldAction>,
}

thread_local! {
    static EVENT_GUARD: RefCell<Option<GuardState>> = const { RefCell::new(None) };
}

pub fn validate_metric_labels<K: AsRef<str>>(labels: &[(K, K)]) -> Result<(), PolicyError> {
    for (key, _) in labels {
        let key_ref = key.as_ref();
        if !ALLOWED_LABEL_SET.contains_key(key_ref) {
            return Err(PolicyError::ForbiddenLabel(key_ref.to_string()));
        }
        if FORBIDDEN_LABEL_REGEX.is_match(key_ref) || PII_FIELD_REGEX.is_match(key_ref) {
            return Err(PolicyError::ForbiddenLabel(key_ref.to_string()));
        }
    }
    Ok(())
}

pub fn contains_pii_key(map: &Map<String, Value>) -> bool {
    map.iter().any(|(key, value)| {
        if PII_FIELD_REGEX.is_match(key) {
            return true;
        }
        if key == "extra" {
            if let Value::Object(nested) = value {
                return contains_pii_key(nested);
            }
        }
        false
    })
}

fn redact_value() -> Value {
    Value::String("[redacted]".to_string())
}

fn scrub_map(map: &mut Map<String, Value>, mode: ScrubMode) -> Result<(), PolicyError> {
    let mut keys_to_redact = Vec::new();
    for key in map.keys() {
        if PII_FIELD_REGEX.is_match(key) {
            keys_to_redact.push(key.clone());
        }
    }
    if !keys_to_redact.is_empty() {
        match mode {
            ScrubMode::Reject => {
                return Err(PolicyError::PiiDetected(keys_to_redact[0].clone()));
            }
            ScrubMode::Redact => {
                for key in keys_to_redact {
                    map.insert(key, redact_value());
                }
            }
        }
    }

    if let Some(extra) = map.get_mut("extra") {
        if let Value::Object(ref mut nested) = extra {
            if let Err(err) = scrub_map(nested, mode) {
                return Err(err);
            }
        }
    }

    Ok(())
}

pub fn scrub_log(mut value: Value, mode: ScrubMode) -> Result<Value, PolicyError> {
    let obj = value
        .as_object_mut()
        .ok_or_else(|| PolicyError::JsonShape("log entry must be a JSON object".into()))?;
    scrub_map(obj, mode)?;
    Ok(value)
}

fn collect_event_fields(event: &Event<'_>) -> HashMap<String, Value> {
    let mut visitor = CollectVisitor::default();
    event.record(&mut visitor);
    visitor.values
}

#[derive(Default)]
struct CollectVisitor {
    values: HashMap<String, Value>,
}

impl tracing_core::field::Visit for CollectVisitor {
    fn record_i64(&mut self, field: &tracing_core::Field, value: i64) {
        self.values
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing_core::Field, value: u64) {
        if let Some(number) = serde_json::Number::from_u128(value as u128) {
            self.values
                .insert(field.name().to_string(), Value::Number(number));
        } else {
            self.values.insert(
                field.name().to_string(),
                Value::String(value.to_string()),
            );
        }
    }

    fn record_bool(&mut self, field: &tracing_core::Field, value: bool) {
        self.values
            .insert(field.name().to_string(), Value::Bool(value));
    }

    fn record_str(&mut self, field: &tracing_core::Field, value: &str) {
        self.values
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_debug(&mut self, field: &tracing_core::Field, value: &dyn std::fmt::Debug) {
        self.values.insert(
            field.name().to_string(),
            Value::String(format!("{:?}", value)),
        );
    }
}

fn plan_field_actions(map: &HashMap<String, Value>) -> Option<HashMap<String, FieldAction>> {
    let mut actions = HashMap::new();
    let mut has_pii = false;
    for key in map.keys() {
        if PII_FIELD_REGEX.is_match(key) {
            actions.insert(key.clone(), FieldAction::Redact(redact_value()));
            has_pii = true;
        }
    }
    if has_pii {
        Some(actions)
    } else {
        None
    }
}

pub fn current_field_action(field: &str) -> Option<FieldAction> {
    EVENT_GUARD.with(|state| {
        state
            .borrow()
            .as_ref()
            .and_then(|s| s.actions.get(field).cloned())
    })
}

fn set_guard_state(actions: Option<HashMap<String, FieldAction>>) {
    EVENT_GUARD.with(|state| {
        *state.borrow_mut() = actions.map(|actions| GuardState { actions });
    });
}

fn clear_guard_state() {
    EVENT_GUARD.with(|state| {
        state.borrow_mut().take();
    });
}

pub struct PiiGuardLayer {
    mode: ScrubMode,
}

impl PiiGuardLayer {
    pub fn new(mode: ScrubMode) -> Self {
        Self { mode }
    }
}

impl<S> Layer<S> for PiiGuardLayer
where
    S: Subscriber,
{
    fn event_enabled(&self, event: &Event<'_>, _ctx: Context<'_, S>) -> bool {
        let fields = collect_event_fields(event);
        match self.mode {
            ScrubMode::Reject => {
                let has_pii = fields.keys().any(|k| PII_FIELD_REGEX.is_match(k));
                if has_pii {
                    clear_guard_state();
                    false
                } else {
                    true
                }
            }
            ScrubMode::Redact => {
                let actions = plan_field_actions(&fields);
                set_guard_state(actions);
                true
            }
        }
    }

    fn on_event(&self, _event: &Event<'_>, _ctx: Context<'_, S>) {
        if matches!(self.mode, ScrubMode::Redact) {
            clear_guard_state();
        }
    }
}
