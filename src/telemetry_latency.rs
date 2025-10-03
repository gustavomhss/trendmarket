use std::fmt;
use std::time::Instant;

pub const OP_REGEX: &str = "^(swap|add_liquidity|remove_liquidity|pricing|cdc_consume)$";
pub const SERVICE_REGEX: &str = "^[a-z0-9._-]{3,64}$";
pub const ENV_REGEX: &str = "^(dev|stg|prod)$";
pub const VERSION_REGEX: &str = "^[A-Za-z0-9+._-]{2,64}$";

#[derive(Debug, Clone)]
pub struct Label {
    pub key: String,
    pub value: String,
}

pub trait LatencySink: Send + Sync {
    fn record(&self, seconds: f64, labels: &[Label]);
}

#[derive(Debug)]
pub enum LatencyError {
    ForbiddenLabel(String),
    InvalidOp(String),
}

impl fmt::Display for LatencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LatencyError::ForbiddenLabel(label) => write!(f, "forbidden label: {label}"),
            LatencyError::InvalidOp(op) => write!(f, "invalid op: {op}"),
        }
    }
}

impl std::error::Error for LatencyError {}

#[derive(Debug)]
pub struct LatencyGuard<'a, S: LatencySink> {
    sink: &'a S,
    start: Instant,
    labels: Vec<Label>,
    recorded: bool,
}

pub fn is_valid_label_key(k: &str) -> bool {
    matches!(k, "op" | "service" | "env" | "version")
}

pub fn is_valid_label_value(v: &str) -> bool {
    is_valid_op(v) || is_valid_service(v) || is_valid_env(v) || is_valid_version(v)
}

impl Label {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl<'a, S: LatencySink> LatencyGuard<'a, S> {
    pub fn new(op: &'static str, base_labels: &[Label], sink: &'a S) -> Result<Self, LatencyError> {
        validate_op(op)?;
        let mut labels = Vec::with_capacity(base_labels.len() + 1);
        let mut has_op_label = false;

        for label in base_labels {
            validate_label(label, op)?;
            if label.key == "op" {
                has_op_label = true;
            }
            labels.push(label.clone());
        }

        if !has_op_label {
            labels.push(Label::new("op", op));
        }

        Ok(Self {
            sink,
            start: Instant::now(),
            labels,
            recorded: false,
        })
    }
}

impl<'a, S: LatencySink> Drop for LatencyGuard<'a, S> {
    fn drop(&mut self) {
        if self.recorded {
            return;
        }
        let seconds = self.start.elapsed().as_secs_f64();
        self.sink.record(seconds, &self.labels);
        self.recorded = true;
    }
}

pub fn guard<'a, S: LatencySink>(
    op: &'static str,
    base_labels: &[Label],
    sink: &'a S,
) -> LatencyGuard<'a, S> {
    LatencyGuard::new(op, base_labels, sink)
        .unwrap_or_else(|err| panic!("latency guard error: {err}"))
}

pub fn with_latency<S: LatencySink, T, F: FnOnce() -> T>(
    op: &'static str,
    base_labels: &[Label],
    sink: &S,
    f: F,
) -> T {
    let guard = LatencyGuard::new(op, base_labels, sink)
        .unwrap_or_else(|err| panic!("with_latency validation error: {err}"));
    let out = f();
    drop(guard);
    out
}

fn validate_op(op: &str) -> Result<(), LatencyError> {
    if is_valid_op(op) {
        Ok(())
    } else {
        Err(LatencyError::InvalidOp(op.to_string()))
    }
}

fn validate_label(label: &Label, op: &str) -> Result<(), LatencyError> {
    if !is_valid_label_key(&label.key) {
        return Err(LatencyError::ForbiddenLabel(label.key.clone()));
    }
    match label.key.as_str() {
        "op" => {
            if !is_valid_op(&label.value) {
                return Err(LatencyError::InvalidOp(label.value.clone()));
            }
            if label.value != op {
                return Err(LatencyError::InvalidOp(label.value.clone()));
            }
            Ok(())
        }
        "service" => {
            if is_valid_service(&label.value) {
                Ok(())
            } else {
                Err(LatencyError::ForbiddenLabel(label.value.clone()))
            }
        }
        "env" => {
            if is_valid_env(&label.value) {
                Ok(())
            } else {
                Err(LatencyError::ForbiddenLabel(label.value.clone()))
            }
        }
        "version" => {
            if is_valid_version(&label.value) {
                Ok(())
            } else {
                Err(LatencyError::ForbiddenLabel(label.value.clone()))
            }
        }
        _ => Err(LatencyError::ForbiddenLabel(label.key.clone())),
    }
}

fn is_valid_op(value: &str) -> bool {
    matches!(
        value,
        "swap" | "add_liquidity" | "remove_liquidity" | "pricing" | "cdc_consume"
    )
}

fn is_valid_service(value: &str) -> bool {
    let len = value.len();
    if !(3..=64).contains(&len) {
        return false;
    }
    value
        .bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'.' | b'_' | b'-'))
}

fn is_valid_env(value: &str) -> bool {
    matches!(value, "dev" | "stg" | "prod")
}

fn is_valid_version(value: &str) -> bool {
    let len = value.len();
    if !(2..=64).contains(&len) {
        return false;
    }
    value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '_' | '-'))
}

#[cfg(test)]
mod internal_tests {
    use super::*;

    #[test]
    fn test_label_value_validation() {
        assert!(is_valid_label_value("dev"));
        assert!(is_valid_label_value("amm.core"));
        assert!(is_valid_label_value("v1.2.3"));
        assert!(is_valid_label_value("swap"));
        assert!(!is_valid_label_value("invalid label"));
    }

    #[test]
    fn test_label_key_validation() {
        for key in ["op", "service", "env", "version"] {
            assert!(is_valid_label_key(key));
        }
        assert!(!is_valid_label_key("tenant"));
    }

    #[test]
    fn validates_op_regex() {
        assert!(validate_op("swap").is_ok());
        assert!(validate_op("pricing").is_ok());
        assert!(validate_op("cdc_consume").is_ok());
        assert!(validate_op("invalid").is_err());
    }

    #[test]
    fn validates_service_values() {
        assert!(is_valid_service("amm.core"));
        assert!(!is_valid_service("Amm.Core"));
    }

    #[test]
    fn validates_version_values() {
        assert!(is_valid_version("v1.0.0"));
        assert!(!is_valid_version("v"));
    }
}
