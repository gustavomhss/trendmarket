use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

use credit_engine_core::telemetry_metrics_otlp::{
    init_meter_otlp, named_meter, select_protocol, MetricsInitError, MetricsOtlpConfig, ObsLevel,
    OtlpProtocol, ResourcePairs,
};
use opentelemetry::KeyValue;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::metrics::{
    data::ResourceMetrics,
    exporter::PushMetricExporter,
    PeriodicReader,
    Temporality,
};
use opentelemetry_sdk::Resource;
use tokio::runtime::Runtime;

#[derive(Clone, Debug)]
struct RecordingExporter {
    exports: Arc<AtomicUsize>,
}

impl RecordingExporter {
    fn new(exports: Arc<AtomicUsize>) -> Self {
        Self { exports }
    }
}

impl PushMetricExporter for RecordingExporter {
    fn export(
        &self,
        _metrics: &mut ResourceMetrics,
    ) -> impl std::future::Future<Output = Result<(), OTelSdkError>> + Send {
        let exports = self.exports.clone();
        async move {
            exports.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn force_flush(&self) -> Result<(), OTelSdkError> {
        Ok(())
    }

    fn shutdown(&self) -> Result<(), OTelSdkError> {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        Temporality::Cumulative
    }
}

fn base_resource() -> ResourcePairs {
    vec![
        ("service.name", "trendmarket-tests".into()),
        ("service.version", "0.0.1-test".into()),
        ("deployment.environment", "test".into()),
    ]
}

#[test]
fn protocol_detection_prefers_explicit_value() {
    assert_eq!(
        select_protocol("http://collector:4317", Some(OtlpProtocol::Http)),
        OtlpProtocol::Http
    );
    assert_eq!(
        select_protocol(
            "https://collector:4318/v1/metrics",
            Some(OtlpProtocol::Grpc),
        ),
        OtlpProtocol::Grpc
    );
}

#[test]
fn protocol_detection_uses_endpoint_hints() {
    assert_eq!(
        select_protocol("http://collector:4318", None),
        OtlpProtocol::Http
    );
    assert_eq!(
        select_protocol("https://collector.example.com/v1/metrics", None),
        OtlpProtocol::Http
    );
    assert_eq!(
        select_protocol("https://collector:4317", None),
        OtlpProtocol::Grpc
    );
}

#[test]
fn resource_validation_rejects_missing_keys() {
    let mut resource = base_resource();
    resource.pop();
    let cfg = MetricsOtlpConfig {
        level: ObsLevel::Off,
        otlp_endpoint: None,
        protocol: None,
        export_interval_ms: 5_000,
        export_timeout_ms: 10_000,
    };
    match init_meter_otlp(cfg, resource) {
        Err(err) => assert!(matches!(err, MetricsInitError::InvalidResource(_))),
        Ok(_) => panic!("expected invalid resource error"),
    }
}

#[test]
fn missing_endpoint_for_active_level_is_an_error() {
    let cfg = MetricsOtlpConfig {
        level: ObsLevel::Min,
        otlp_endpoint: None,
        protocol: None,
        export_interval_ms: 5_000,
        export_timeout_ms: 10_000,
    };
    match init_meter_otlp(cfg, base_resource()) {
        Err(MetricsInitError::MissingEndpointForActiveLevel) => {}
        Err(other) => panic!("unexpected error: {other:?}"),
        Ok(_) => panic!("expected missing endpoint error"),
    }
}

#[test]
fn metrics_guard_shutdown_is_idempotent() {
    let cfg = MetricsOtlpConfig {
        level: ObsLevel::Off,
        otlp_endpoint: None,
        protocol: None,
        export_interval_ms: 5_000,
        export_timeout_ms: 10_000,
    };
    let (mut guard, provider) = init_meter_otlp(cfg, base_resource()).unwrap();
    guard.shutdown();
    drop(guard);
    match provider.shutdown() {
        Ok(()) | Err(OTelSdkError::AlreadyShutdown) => {}
        Err(other) => panic!("unexpected shutdown error: {other:?}"),
    }
}

#[test]
fn periodic_reader_exports_metrics_to_recording_exporter() {
    let runtime = Runtime::new().expect("tokio runtime");
    let exports = Arc::new(AtomicUsize::new(0));
    let exporter = RecordingExporter::new(exports.clone());

    runtime.block_on(async {
        let reader: PeriodicReader<RecordingExporter> = PeriodicReader::builder(exporter)
            .with_interval(Duration::from_millis(50))
            .build();

        let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
            .with_resource(
                Resource::builder()
                    .with_attributes(vec![
                        KeyValue::new("service.name", "trendmarket-tests"),
                        KeyValue::new("service.version", "0.0.1-test"),
                        KeyValue::new("deployment.environment", "test"),
                    ])
                    .build(),
            )
            .with_reader(reader)
            .build();

        let meter = named_meter(&provider, "periodic_test");
        let counter = meter
            .u64_counter("periodic_counter")
            .with_description("counts periodic writes")
            .build();
        counter.add(1, &[]);

        tokio::time::sleep(Duration::from_millis(150)).await;

        provider.shutdown().expect("shutdown succeeds");
    });

    assert!(
        exports.load(Ordering::SeqCst) > 0,
        "expected at least one export"
    );
}
