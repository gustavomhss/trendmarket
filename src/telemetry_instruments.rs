use once_cell::sync::Lazy;
use opentelemetry::metrics::{Counter, Histogram, Meter, ObservableGauge};
use regex::Regex;

/// Canonical histogram buckets for AMM operation latency in seconds.
const AMM_OP_LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0,
];

const AMM_OP_LATENCY_NAME: &str = "amm_op_latency_seconds";
const AMM_OP_LATENCY_DESCRIPTION: &str = "Latency per AMM operation in seconds";
const AMM_OP_LATENCY_UNIT: &str = "s";

const HOOK_EXECUTIONS_NAME: &str = "hook_executions_total";
const HOOK_EXECUTIONS_DESCRIPTION: &str = "Hook executions partitioned by id and status";

const DATA_FRESHNESS_NAME: &str = "data_freshness_seconds";
const DATA_FRESHNESS_DESCRIPTION: &str = "Data freshness per source and domain";
const DATA_FRESHNESS_UNIT: &str = "s";

const CDC_LAG_NAME: &str = "cdc_lag_seconds";
const CDC_LAG_DESCRIPTION: &str = "Change data capture lag per stream and partition";
const CDC_LAG_UNIT: &str = "s";

const DRIFT_SCORE_NAME: &str = "drift_score";
const DRIFT_SCORE_DESCRIPTION: &str = "Feature drift score (0..1 expected) per feature and domain";
const DRIFT_SCORE_UNIT: &str = "1";

const ALLOWED_LABELS: &[&str] = &["op", "service", "env", "version"];
const FORBIDDEN_EXACT_LABELS: &[&str] = &["user_id", "account_id", "request_id", "session_id"];

static FORBIDDEN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(email|cpf|phone|address|name|geo|person_)")
        .expect("valid forbidden label regex")
});

/// Canonical AMM metric instruments registered from a provided meter.
pub struct AmmMetrics {
    pub latency_hist: Histogram<f64>,
    pub hook_execs: Counter<u64>,
    pub data_freshness: ObservableGauge<f64>,
    pub cdc_lag: ObservableGauge<f64>,
    pub drift_score: ObservableGauge<f64>,
}

/// Registers the canonical AMM metrics using the provided meter and returns the initialized instruments.
pub fn register_amm_metrics(meter: &Meter) -> AmmMetrics {
    let latency_hist = meter
        .f64_histogram(AMM_OP_LATENCY_NAME)
        .with_unit(AMM_OP_LATENCY_UNIT)
        .with_description(AMM_OP_LATENCY_DESCRIPTION)
        .with_boundaries(AMM_OP_LATENCY_BUCKETS.to_vec())
        .build();

    let hook_execs = meter
        .u64_counter(HOOK_EXECUTIONS_NAME)
        .with_description(HOOK_EXECUTIONS_DESCRIPTION)
        .build();

    let data_freshness = meter
        .f64_observable_gauge(DATA_FRESHNESS_NAME)
        .with_unit(DATA_FRESHNESS_UNIT)
        .with_description(DATA_FRESHNESS_DESCRIPTION)
        .build();

    let cdc_lag = meter
        .f64_observable_gauge(CDC_LAG_NAME)
        .with_unit(CDC_LAG_UNIT)
        .with_description(CDC_LAG_DESCRIPTION)
        .build();

    let drift_score = meter
        .f64_observable_gauge(DRIFT_SCORE_NAME)
        .with_unit(DRIFT_SCORE_UNIT)
        .with_description(DRIFT_SCORE_DESCRIPTION)
        .build();

    AmmMetrics {
        latency_hist,
        hook_execs,
        data_freshness,
        cdc_lag,
        drift_score,
    }
}

/// Returns the whitelist of permitted attribute keys for AMM metrics.
pub fn allowed_labels() -> &'static [&'static str] {
    ALLOWED_LABELS
}

/// Returns true when the provided label key is explicitly allowed by the contract.
pub fn is_label_allowed(key: &str) -> bool {
    ALLOWED_LABELS.iter().any(|allowed| key == *allowed)
}

/// Returns true when the provided label key is forbidden either by exact match or policy heuristics.
pub fn is_label_forbidden(key: &str) -> bool {
    let lowercase = key.to_ascii_lowercase();

    if ALLOWED_LABELS.iter().any(|allowed| lowercase == *allowed) {
        return false;
    }

    if FORBIDDEN_EXACT_LABELS
        .iter()
        .any(|forbidden| lowercase == *forbidden)
    {
        return true;
    }

    if lowercase.ends_with("_uuid") || lowercase.ends_with("_hash") {
        return true;
    }

    FORBIDDEN_PATTERN.is_match(&lowercase)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_labels_list_is_canonical() {
        assert_eq!(allowed_labels(), &["op", "service", "env", "version"]);
    }

    #[test]
    fn allowed_and_forbidden_label_checks_behave() {
        for key in allowed_labels() {
            assert!(is_label_allowed(key));
            assert!(!is_label_forbidden(key));
        }

        for key in ["user_id", "account_id", "request_id", "session_id"] {
            assert!(is_label_forbidden(key));
            assert!(!is_label_allowed(key));
        }

        assert!(is_label_forbidden("customer_email"));
        assert!(is_label_forbidden("payment_uuid"));
        assert!(is_label_forbidden("device_hash"));
        assert!(!is_label_forbidden("custom_label"));
    }
}
