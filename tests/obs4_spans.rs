use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use credit_engine_core::obs4::spans::{
    set_status_from_result, span_amm_swap, GuardrailEvent, SwapReq,
};
use opentelemetry::trace::{Status, TracerProvider};
use opentelemetry_sdk::error::OTelSdkResult;
use opentelemetry_sdk::trace::{SdkTracerProvider, SpanData, SpanExporter};
use tracing::field::{Field, Visit};
use tracing::Id;
use tracing::{span, Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::{layer::SubscriberExt, Registry};

#[test]
fn span_amm_swap_has_canonical_name() {
    let req = SwapReq {
        k_before: 1.0,
        k_after: 2.0,
        delta_k_ratio: 0.5,
        fee_ppm: 100,
        input_amount: 10.0,
        output_amount: 9.5,
        asset_in: "USDC",
        asset_out: "BRL",
        guardrail_hit: false,
        guardrail: None,
        rounding_rule: None,
    };

    let span = span_amm_swap(&req);
    assert_eq!(span.metadata().map(|m| m.name()), Some("amm.swap"));
}

#[test]
fn span_amm_swap_applies_attributes() {
    let layer = CollectingLayer::default();
    let subscriber = Registry::default().with(layer.clone());

    tracing::subscriber::with_default(subscriber, || {
        let req = SwapReq {
            k_before: 100.0,
            k_after: 101.0,
            delta_k_ratio: 0.01,
            fee_ppm: 250,
            input_amount: 1_000.0,
            output_amount: 995.0,
            asset_in: "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            asset_out: "BRL-POOL-2024Q4-LONG",
            guardrail_hit: false,
            guardrail: None,
            rounding_rule: Some("round-to-nearest"),
        };

        let span = span_amm_swap(&req);
        drop(span);
    });

    let spans = layer.spans();
    let recorded = spans
        .iter()
        .find(|span| span.name == "amm.swap")
        .expect("span recorded");

    assert_eq!(
        recorded
            .fields
            .get("amm.asset_in")
            .and_then(RecordedValue::as_str),
        Some("ABCDEFGHIJKLMNOP")
    );
    assert_eq!(
        recorded
            .fields
            .get("amm.asset_out")
            .and_then(RecordedValue::as_str),
        Some("BRL-POOL-2024Q4-")
    );
    assert_eq!(
        recorded
            .fields
            .get("amm.guardrail_hit")
            .and_then(RecordedValue::as_bool),
        Some(false)
    );
}

#[test]
fn span_amm_swap_emits_guardrail_event() {
    let layer = CollectingLayer::default();
    let subscriber = Registry::default().with(layer.clone());

    tracing::subscriber::with_default(subscriber, || {
        let req = SwapReq {
            k_before: 200.0,
            k_after: 199.0,
            delta_k_ratio: -0.005,
            fee_ppm: 300,
            input_amount: 5_000.0,
            output_amount: 0.0,
            asset_in: "USDC",
            asset_out: "BRL",
            guardrail_hit: true,
            guardrail: Some(GuardrailEvent {
                code: "CE-AMM-GRD-001",
                reason: "limit breached",
            }),
            rounding_rule: None,
        };

        let span = span_amm_swap(&req);
        drop(span);
    });

    let events = layer.events();
    let guardrail_event = events
        .iter()
        .find(|event| {
            event
                .fields
                .get("EVENT_VALIDATION_FAILED")
                .and_then(RecordedValue::as_str)
                == Some("amm.validation_failed")
        })
        .expect("guardrail event present");

    assert_eq!(
        guardrail_event
            .fields
            .get("code")
            .and_then(RecordedValue::as_str),
        Some("CE-AMM-GRD-001")
    );
    assert_eq!(
        guardrail_event
            .fields
            .get("reason")
            .and_then(RecordedValue::as_str),
        Some("limit breached")
    );
}

#[test]
fn set_status_from_result_marks_error() {
    let (exporter, exported) = CollectingExporter::new();
    let provider = SdkTracerProvider::builder()
        .with_simple_exporter(exporter)
        .build();
    let tracer = provider.tracer("obs4_spans_test");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(otel_layer);

    tracing::subscriber::with_default(subscriber, || {
        let req = SwapReq {
            k_before: 10.0,
            k_after: 9.0,
            delta_k_ratio: -0.1,
            fee_ppm: 400,
            input_amount: 100.0,
            output_amount: 0.0,
            asset_in: "USDC",
            asset_out: "BRL",
            guardrail_hit: true,
            guardrail: None,
            rounding_rule: None,
        };

        let span = span_amm_swap(&req);
        let result: Result<(), TestError> = Err(TestError("failure"));
        set_status_from_result(&span, &result);
        drop(span);
    });

    assert!(provider.force_flush().is_ok(), "force_flush error");
    assert!(provider.shutdown().is_ok(), "shutdown error");

    let spans = exported.lock().expect("exported spans lock");
    assert!(spans
        .iter()
        .any(|span| matches!(span.status, Status::Error { .. })));
}

#[derive(Clone, Debug, Default)]
struct CollectingLayer {
    spans: Arc<Mutex<HashMap<Id, RecordedSpan>>>,
    events: Arc<Mutex<Vec<RecordedEvent>>>,
}

impl CollectingLayer {
    fn spans(&self) -> Vec<RecordedSpan> {
        self.spans
            .lock()
            .expect("span lock")
            .values()
            .cloned()
            .collect()
    }

    fn events(&self) -> Vec<RecordedEvent> {
        self.events.lock().expect("event lock").clone()
    }
}

#[derive(Clone, Debug, Default)]
struct RecordedSpan {
    name: String,
    fields: HashMap<String, RecordedValue>,
}

#[derive(Clone, Debug, Default)]
struct RecordedEvent {
    fields: HashMap<String, RecordedValue>,
}

#[derive(Clone, Debug, PartialEq)]
enum RecordedValue {
    F64(f64),
    I64(i64),
    U64(u64),
    Bool(bool),
    Str(String),
    None,
}

impl Default for RecordedValue {
    fn default() -> Self {
        RecordedValue::None
    }
}

impl RecordedValue {
    fn as_str(&self) -> Option<&str> {
        match self {
            RecordedValue::Str(value) => Some(value.as_str()),
            _ => None,
        }
    }

    fn as_bool(&self) -> Option<bool> {
        match self {
            RecordedValue::Bool(value) => Some(*value),
            _ => None,
        }
    }
}

impl<S> Layer<S> for CollectingLayer
where
    S: Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &Id, _ctx: Context<'_, S>) {
        let mut fields = HashMap::new();
        let mut visitor = FieldRecorder::new(&mut fields);
        attrs.record(&mut visitor);
        let span = RecordedSpan {
            name: attrs.metadata().name().to_string(),
            fields,
        };
        self.spans
            .lock()
            .expect("span lock")
            .insert(id.clone(), span);
    }

    fn on_record(&self, id: &Id, values: &span::Record<'_>, _ctx: Context<'_, S>) {
        if let Some(span) = self.spans.lock().expect("span lock").get_mut(id) {
            let mut visitor = FieldRecorder::new(&mut span.fields);
            values.record(&mut visitor);
        }
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut fields = HashMap::new();
        let mut visitor = FieldRecorder::new(&mut fields);
        event.record(&mut visitor);
        self.events
            .lock()
            .expect("event lock")
            .push(RecordedEvent { fields });
    }
}

struct FieldRecorder<'a> {
    fields: &'a mut HashMap<String, RecordedValue>,
}

impl<'a> FieldRecorder<'a> {
    fn new(fields: &'a mut HashMap<String, RecordedValue>) -> Self {
        Self { fields }
    }

    fn insert(&mut self, field: &Field, value: RecordedValue) {
        self.fields.insert(field.name().to_string(), value);
    }
}

impl<'a> Visit for FieldRecorder<'a> {
    fn record_f64(&mut self, field: &Field, value: f64) {
        self.insert(field, RecordedValue::F64(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, RecordedValue::I64(value));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, RecordedValue::U64(value));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, RecordedValue::Bool(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field, RecordedValue::Str(value.to_string()));
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let rendered = format!("{value:?}");
        if rendered == "tracing::field::Empty" {
            return;
        }
        let sanitized = rendered.trim_matches('"').to_string();
        self.insert(field, RecordedValue::Str(sanitized));
    }
}

#[derive(Debug)]
struct CollectingExporter {
    buffer: Arc<Mutex<Vec<SpanData>>>,
}

impl CollectingExporter {
    fn new() -> (Self, Arc<Mutex<Vec<SpanData>>>) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                buffer: buffer.clone(),
            },
            buffer,
        )
    }
}

impl SpanExporter for CollectingExporter {
    fn export(
        &self,
        batch: Vec<SpanData>,
    ) -> impl std::future::Future<Output = OTelSdkResult> + Send {
        let buffer = self.buffer.clone();
        async move {
            let mut guard = buffer.lock().expect("export buffer lock");
            guard.extend(batch);
            Ok(())
        }
    }
}

#[derive(Debug)]
struct TestError(&'static str);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TestError {}
