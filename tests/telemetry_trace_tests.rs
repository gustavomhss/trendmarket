use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use credit_engine_core::telemetry_trace::{
    init_tracing, init_tracing_with_exporter, sampler_for_level, select_protocol, ObsLevel,
    OtlpProtocol, ResourcePairs, TraceConfig, TraceInitError,
};
use opentelemetry::baggage::BaggageExt;
use opentelemetry::propagation::{Extractor, Injector};
use opentelemetry::testing::trace::TestSpan;
use opentelemetry::trace::{SpanContext, TraceContextExt, TraceFlags, TraceId, TraceState};
use opentelemetry::Context;
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{Sampler, SpanData, SpanExporter};
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[test]
fn protocol_selection_heuristics() {
    assert_eq!(
        select_protocol("http://otel:4317", None),
        OtlpProtocol::Grpc
    );
    assert_eq!(
        select_protocol("https://otel:4318", None),
        OtlpProtocol::Http
    );
    assert_eq!(
        select_protocol("https://otel-collector/v1/traces", None),
        OtlpProtocol::Http
    );
    assert_eq!(
        select_protocol("grpc://otel:4317", Some(OtlpProtocol::Grpc)),
        OtlpProtocol::Grpc
    );
}

#[test]
fn sampler_mapping_matches_contract() {
    match sampler_for_level(ObsLevel::Off) {
        Sampler::AlwaysOff => {}
        other => panic!("unexpected sampler for Off: {other:?}"),
    }

    match sampler_for_level(ObsLevel::Min) {
        Sampler::ParentBased(inner) => match inner.as_ref() {
            Sampler::TraceIdRatioBased(prob) => assert!((prob - 0.01).abs() < f64::EPSILON),
            other => panic!("unexpected delegate sampler for Min: {other:?}"),
        },
        other => panic!("unexpected sampler for Min: {other:?}"),
    }

    match sampler_for_level(ObsLevel::Full) {
        Sampler::ParentBased(inner) => match inner.as_ref() {
            Sampler::AlwaysOn => {}
            other => panic!("unexpected delegate sampler for Full: {other:?}"),
        },
        other => panic!("unexpected sampler for Full: {other:?}"),
    }
}

#[test]
fn propagator_registration_is_idempotent() -> Result<(), TraceInitError> {
    let cfg = TraceConfig {
        level: ObsLevel::Off,
        ..TraceConfig::default()
    };

    let resource = test_resource();
    let (mut guard1, _layer1) = init_tracing(cfg.clone(), resource.clone())?;
    guard1.shutdown();
    drop(guard1);

    let (mut guard2, _layer2) = init_tracing(cfg, resource)?;
    guard2.shutdown();

    let trace_id = TraceId::from_u128(42);
    let span_id = opentelemetry::trace::SpanId::from_u64(7);
    let span_context = SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::SAMPLED,
        false,
        TraceState::default(),
    );
    let baggage = opentelemetry::baggage::Baggage::builder()
        .with_kv("feature", "enabled")
        .build();
    let ctx = Context::current()
        .with_span(TestSpan(span_context))
        .with_baggage(baggage);

    let mut carrier = HeaderCarrier::default();
    opentelemetry::global::get_text_map_propagator(|prop| {
        prop.inject_context(&ctx, &mut carrier);
    });

    assert!(carrier.0.contains_key("traceparent"));
    let baggage_header = carrier
        .0
        .get("baggage")
        .expect("baggage header should be present");
    assert!(baggage_header.contains("feature=enabled"));

    Ok(())
}

#[test]
fn spans_export_through_custom_exporter() -> Result<(), TraceInitError> {
    let exported: Arc<Mutex<Vec<SpanData>>> = Arc::new(Mutex::new(Vec::new()));
    let exporter = RecordingExporter::new(exported.clone());

    let cfg = TraceConfig {
        level: ObsLevel::Full,
        otlp_endpoint: None,
        protocol: Some(OtlpProtocol::Grpc),
        ..TraceConfig::default()
    };

    let resource = test_resource();
    let (mut guard, layer) = init_tracing_with_exporter(cfg, resource, exporter)?;
    let subscriber = Registry::default().with(layer);

    tracing::subscriber::with_default(subscriber, || {
        let span = tracing::info_span!("export-test-span");
        span.in_scope(|| {});
    });

    guard.shutdown();

    let spans = exported.lock().expect("exported spans lock");
    assert_eq!(spans.len(), 1, "expected a single exported span");

    Ok(())
}

#[test]
fn trace_guard_shutdown_is_idempotent() -> Result<(), TraceInitError> {
    let exported: Arc<Mutex<Vec<SpanData>>> = Arc::new(Mutex::new(Vec::new()));
    let exporter = RecordingExporter::new(exported);
    let cfg = TraceConfig {
        level: ObsLevel::Full,
        otlp_endpoint: None,
        protocol: Some(OtlpProtocol::Http),
        ..TraceConfig::default()
    };
    let resource = test_resource();
    let (mut guard, _layer) = init_tracing_with_exporter(cfg, resource, exporter)?;
    guard.shutdown();
    guard.shutdown();
    Ok(())
}

fn test_resource() -> ResourcePairs {
    vec![
        ("service.name", "ce-amm".to_string()),
        ("service.version", "0.0.0-test".to_string()),
        ("deployment.environment", "dev".to_string()),
    ]
}

#[derive(Default)]
struct HeaderCarrier(HashMap<String, String>);

impl Extractor for HeaderCarrier {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(|value| value.as_str())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|key| key.as_str()).collect()
    }
}

impl Injector for HeaderCarrier {
    fn set(&mut self, key: &str, value: String) {
        self.0.insert(key.to_string(), value);
    }
}

#[derive(Clone)]
struct RecordingExporter {
    spans: Arc<Mutex<Vec<SpanData>>>,
}

impl RecordingExporter {
    fn new(spans: Arc<Mutex<Vec<SpanData>>>) -> Self {
        Self { spans }
    }
}

impl fmt::Debug for RecordingExporter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecordingExporter").finish()
    }
}

impl SpanExporter for RecordingExporter {
    fn export(&self, batch: Vec<SpanData>) -> Pin<Box<dyn Future<Output = OTelSdkResult> + Send>> {
        let spans = self.spans.clone();
        Box::pin(async move {
            let mut guard = spans.lock().expect("exported spans lock");
            guard.extend(batch);
            Ok(())
        })
    }
}
