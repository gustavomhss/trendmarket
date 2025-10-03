use std::sync::{Arc, Mutex};
use std::time::Duration;

use credit_engine_core::telemetry_latency::{
    guard, is_valid_label_key, is_valid_label_value, with_latency, Label, LatencyError,
    LatencyGuard, LatencySink,
};

#[derive(Debug, Clone, Default)]
struct MockSink {
    inner: Arc<Mutex<Vec<(f64, Vec<Label>)>>>,
}

impl MockSink {
    fn records(&self) -> Vec<(f64, Vec<Label>)> {
        self.inner.lock().expect("records lock poisoned").clone()
    }
}

impl LatencySink for MockSink {
    fn record(&self, seconds: f64, labels: &[Label]) {
        self.inner
            .lock()
            .expect("record lock poisoned")
            .push((seconds, labels.to_vec()));
    }
}

#[test]
fn wrapper_records_latency_and_labels() {
    let sink = MockSink::default();
    let base_labels = vec![
        Label::new("service", "amm.core"),
        Label::new("env", "dev"),
        Label::new("version", "v1.0.0"),
    ];
    let expected = 7u32;
    let result = with_latency("swap", &base_labels, &sink, || {
        std::thread::sleep(Duration::from_millis(2));
        expected
    });
    assert_eq!(result, expected);

    let records = sink.records();
    assert_eq!(records.len(), 1);
    let (seconds, labels) = &records[0];
    assert!(
        *seconds > 0.0,
        "expected elapsed seconds > 0, got {seconds}"
    );
    assert_eq!(labels.len(), 4);
    assert!(labels.iter().any(|l| l.key == "op" && l.value == "swap"));
    for required in ["service", "env", "version"] {
        assert!(labels.iter().any(|l| l.key == required));
    }
    println!(
        "OBS1_MOCK_SAMPLE seconds={:.6} labels={:?}",
        seconds, labels
    );
}

#[test]
fn guard_records_latency_via_drop() {
    let sink = MockSink::default();
    let base_labels = vec![Label::new("service", "amm.core")];
    {
        let _g = guard("pricing", &base_labels, &sink);
        std::thread::sleep(Duration::from_millis(1));
    }
    let records = sink.records();
    assert_eq!(records.len(), 1);
    let (seconds, labels) = &records[0];
    assert!(*seconds > 0.0);
    assert!(labels.iter().any(|l| l.key == "op" && l.value == "pricing"));
}

#[test]
fn guard_allows_existing_op_label() {
    let sink = MockSink::default();
    let base_labels = vec![
        Label::new("service", "amm.core"),
        Label::new("op", "add_liquidity"),
    ];
    let guard =
        LatencyGuard::new("add_liquidity", &base_labels, &sink).expect("guard should be created");
    drop(guard);

    let records = sink.records();
    assert_eq!(records.len(), 1);
    let (_, labels) = &records[0];
    let op_count = labels.iter().filter(|l| l.key == "op").count();
    assert_eq!(op_count, 1);
}

#[test]
fn rejects_forbidden_label_key() {
    let sink = MockSink::default();
    let labels = vec![Label::new("tenant", "demo")];
    let err = LatencyGuard::new("swap", &labels, &sink).expect_err("should reject forbidden label");
    match err {
        LatencyError::ForbiddenLabel(ref key) => assert_eq!(key, "tenant"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn rejects_invalid_label_value() {
    let sink = MockSink::default();
    let labels = vec![Label::new("service", "InvalidCaps")];
    let err = LatencyGuard::new("swap", &labels, &sink).expect_err("should reject invalid value");
    matches!(err, LatencyError::ForbiddenLabel(_));
}

#[test]
fn rejects_invalid_op_value() {
    let sink = MockSink::default();
    let err = LatencyGuard::new("unknown", &[], &sink).expect_err("invalid op");
    match err {
        LatencyError::InvalidOp(op) => assert_eq!(op, "unknown"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn helper_validation_functions_match_policy() {
    assert!(is_valid_label_key("service"));
    assert!(is_valid_label_value("amm.core"));
    assert!(is_valid_label_value("prod"));
    assert!(!is_valid_label_value("bad value"));
}
