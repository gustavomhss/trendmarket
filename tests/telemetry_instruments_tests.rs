use credit_engine_core::telemetry_instruments::{
    allowed_labels, is_label_allowed, is_label_forbidden, register_amm_metrics,
};
use opentelemetry::{metrics::MeterProvider as _, KeyValue};
use opentelemetry_sdk::metrics::{
    data::Histogram as HistogramData, data::ResourceMetrics, InMemoryMetricExporter,
    PeriodicReader, SdkMeterProvider,
};

fn setup_meter() -> (SdkMeterProvider, InMemoryMetricExporter) {
    let exporter = InMemoryMetricExporter::default();
    let provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(exporter.clone()).build())
        .build();
    (provider, exporter)
}

fn collect(provider: &SdkMeterProvider, exporter: &InMemoryMetricExporter) -> Vec<ResourceMetrics> {
    provider.force_flush().expect("force flush metrics");
    exporter
        .get_finished_metrics()
        .expect("retrieve finished metrics")
}

#[test]
fn registers_histogram_and_counter_with_expected_metadata() {
    let (provider, exporter) = setup_meter();
    let meter = provider.meter("obs1-test");
    let instruments = register_amm_metrics(&meter);

    instruments.latency_hist.record(
        0.075,
        &[
            KeyValue::new("op", "swap"),
            KeyValue::new("service", "ce-amm"),
        ],
    );
    instruments.hook_execs.add(
        1,
        &[
            KeyValue::new("hook_id", "h1"),
            KeyValue::new("status", "success"),
        ],
    );

    let metrics = collect(&provider, &exporter);
    let mut names = vec![];

    for resource_metrics in metrics {
        for scope in resource_metrics.scope_metrics {
            for metric in scope.metrics {
                let name = metric.name.to_string();
                if name == "amm_op_latency_seconds" {
                    assert_eq!(
                        metric.description.as_ref(),
                        "Latency per AMM operation in seconds"
                    );
                    assert_eq!(metric.unit.as_ref(), "s");
                }
                if name == "hook_executions_total" {
                    assert_eq!(
                        metric.description.as_ref(),
                        "Hook executions partitioned by id and status"
                    );
                }
                names.push(name);
            }
        }
    }

    assert!(names.contains(&"amm_op_latency_seconds".to_string()));
    assert!(names.contains(&"hook_executions_total".to_string()));
}

#[test]
fn latency_histogram_records_sample_with_allowed_labels() {
    let (provider, exporter) = setup_meter();
    let meter = provider.meter("obs1-test");
    let instruments = register_amm_metrics(&meter);

    instruments.latency_hist.record(
        0.012,
        &[
            KeyValue::new("op", "swap"),
            KeyValue::new("service", "ce-amm"),
            KeyValue::new("env", "dev"),
            KeyValue::new("version", "0.0.0+devhash"),
        ],
    );

    let metrics = collect(&provider, &exporter);

    let mut found = false;
    for resource_metrics in metrics {
        for scope in resource_metrics.scope_metrics {
            for metric in scope.metrics {
                if metric.name.as_ref() == "amm_op_latency_seconds" {
                    let hist = metric
                        .data
                        .as_any()
                        .downcast_ref::<HistogramData<f64>>()
                        .expect("histogram data");
                    if let Some(point) = hist.data_points.first() {
                        assert!(point.count >= 1);
                        assert_eq!(
                            point.bounds,
                            vec![
                                0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.5,
                                0.75, 1.0, 1.5, 2.0, 3.0, 5.0,
                            ]
                        );
                        found = true;
                    }
                }
            }
        }
    }

    assert!(found, "histogram data point not found");
}

#[test]
fn label_helpers_enforce_contract() {
    assert_eq!(allowed_labels(), &["op", "service", "env", "version"]);

    for key in allowed_labels() {
        assert!(is_label_allowed(key));
        assert!(!is_label_forbidden(key));
    }

    for forbidden in [
        "user_id",
        "account_id",
        "request_id",
        "session_id",
        "customer_email",
        "device_hash",
    ] {
        assert!(is_label_forbidden(forbidden));
        assert!(!is_label_allowed(forbidden));
    }

    assert!(is_label_forbidden("person_identifier"));
    assert!(is_label_forbidden("loan_uuid"));
    assert!(!is_label_forbidden("custom_tag"));
}
