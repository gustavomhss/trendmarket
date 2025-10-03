use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::{Arc, Mutex};

use credit_engine_core::telemetry_spans_amm::{
    in_amm_add_liquidity, in_amm_remove_liquidity, in_amm_swap, in_pricing_quote,
    span_amm_add_liquidity, span_amm_remove_liquidity, span_amm_swap, span_pricing_quote,
    AddLiquidityAttrs, PricingQuoteAttrs, RemoveLiquidityAttrs, SwapAttrs,
};
use tracing::{span::Id, subscriber::with_default};
use tracing_subscriber::{layer::Context, layer::SubscriberExt, registry::Registry, Layer};

#[derive(Debug, Clone, PartialEq)]
struct CapturedSpan {
    name: String,
    fields: BTreeMap<String, CapturedValue>,
}

#[derive(Debug, Clone, PartialEq)]
enum CapturedValue {
    F64(f64),
    I64(i64),
    Str(String),
    Bool(bool),
}

#[derive(Clone, Default)]
struct CapturingLayer {
    inner: Arc<CapturingInner>,
}

#[derive(Default)]
struct CapturingInner {
    open: Mutex<HashMap<Id, CapturedSpan>>,
    completed: Mutex<Vec<CapturedSpan>>,
}

impl CapturingLayer {
    fn new() -> Self {
        Self {
            inner: Arc::new(CapturingInner::default()),
        }
    }

    fn finished_spans(&self) -> Vec<CapturedSpan> {
        self.inner
            .completed
            .lock()
            .expect("lock completed spans")
            .clone()
    }
}

struct FieldRecorder<'a> {
    fields: &'a mut BTreeMap<String, CapturedValue>,
}

impl<'a> tracing::field::Visit for FieldRecorder<'a> {
    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), CapturedValue::F64(value));
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), CapturedValue::I64(value));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            CapturedValue::Str(value.to_string()),
        );
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), CapturedValue::Bool(value));
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            CapturedValue::Str(format!("{:?}", value)),
        );
    }
}

impl<S> Layer<S> for CapturingLayer
where
    S: tracing::Subscriber,
{
    fn on_new_span(&self, attrs: &tracing::span::Attributes<'_>, id: &Id, _: Context<'_, S>) {
        let mut fields = BTreeMap::new();
        attrs.record(&mut FieldRecorder {
            fields: &mut fields,
        });
        let span = CapturedSpan {
            name: attrs.metadata().name().to_string(),
            fields,
        };
        self.inner
            .open
            .lock()
            .expect("lock open spans")
            .insert(id.clone(), span);
    }

    fn on_record(&self, id: &Id, values: &tracing::span::Record<'_>, _: Context<'_, S>) {
        if let Some(span) = self.inner.open.lock().expect("lock open spans").get_mut(id) {
            values.record(&mut FieldRecorder {
                fields: &mut span.fields,
            });
        }
    }

    fn on_close(&self, id: Id, _: Context<'_, S>) {
        if let Some(span) = self.inner.open.lock().expect("lock open spans").remove(&id) {
            self.inner
                .completed
                .lock()
                .expect("lock completed spans")
                .push(span);
        }
    }
}

fn with_capturing_layer<F>(f: F) -> Vec<CapturedSpan>
where
    F: FnOnce(),
{
    let layer = CapturingLayer::new();
    let subscriber = Registry::default().with(layer.clone());
    with_default(subscriber, || {
        f();
    });
    layer.finished_spans()
}

fn expect_common_fields(span: &CapturedSpan, op: &str, attrs: &SwapAttrs) {
    assert_eq!(
        span.fields.get("op"),
        Some(&CapturedValue::Str(op.to_string()))
    );
    assert_eq!(
        span.fields.get("amm.k_before"),
        Some(&CapturedValue::F64(attrs.k_before))
    );
    assert_eq!(
        span.fields.get("amm.k_after"),
        Some(&CapturedValue::F64(attrs.k_after))
    );
    assert_eq!(
        span.fields.get("amm.delta_k_ratio"),
        Some(&CapturedValue::F64(attrs.delta_k_ratio))
    );
    assert_eq!(
        span.fields.get("amm.fee_ppm"),
        Some(&CapturedValue::I64(attrs.fee_ppm))
    );
    assert_eq!(
        span.fields.get("amm.input"),
        Some(&CapturedValue::F64(attrs.input))
    );
    assert_eq!(
        span.fields.get("amm.output"),
        Some(&CapturedValue::F64(attrs.output))
    );
}

#[test]
fn swap_span_exports_expected_attributes() {
    let attrs = SwapAttrs {
        k_before: 1.0,
        k_after: 1.05,
        delta_k_ratio: 0.05,
        fee_ppm: 300,
        input: 100.0,
        output: 99.7,
    };

    let spans = with_capturing_layer(|| {
        let span = span_amm_swap(&attrs);
        let _guard = span.enter();
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "amm.swap");
    expect_common_fields(span, "swap", &attrs);
}

#[test]
fn swap_wrapper_runs_inside_span() {
    let attrs = SwapAttrs {
        k_before: 2.0,
        k_after: 2.1,
        delta_k_ratio: 0.05,
        fee_ppm: 120,
        input: 50.0,
        output: 49.94,
    };

    let spans = with_capturing_layer(|| {
        let value = in_amm_swap(&attrs, || 42);
        assert_eq!(value, 42);
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "amm.swap");
    expect_common_fields(span, "swap", &attrs);
}

#[test]
fn add_liquidity_span_exports_expected_attributes() {
    let attrs = AddLiquidityAttrs {
        k_before: 5.0,
        k_after: 5.5,
        delta_k_ratio: 0.1,
        fee_ppm: 45,
        input: 1000.0,
        output: 1000.0,
    };

    let spans = with_capturing_layer(|| {
        let span = span_amm_add_liquidity(&attrs);
        let _guard = span.enter();
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "amm.add_liquidity");
    expect_common_fields(
        span,
        "add_liquidity",
        &SwapAttrs {
            k_before: attrs.k_before,
            k_after: attrs.k_after,
            delta_k_ratio: attrs.delta_k_ratio,
            fee_ppm: attrs.fee_ppm,
            input: attrs.input,
            output: attrs.output,
        },
    );
}

#[test]
fn add_liquidity_wrapper_exports_expected_attributes() {
    let attrs = AddLiquidityAttrs {
        k_before: 3.0,
        k_after: 3.3,
        delta_k_ratio: 0.1,
        fee_ppm: 60,
        input: 500.0,
        output: 499.5,
    };

    let spans = with_capturing_layer(|| {
        let result = in_amm_add_liquidity(&attrs, || "ok");
        assert_eq!(result, "ok");
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "amm.add_liquidity");
    expect_common_fields(
        span,
        "add_liquidity",
        &SwapAttrs {
            k_before: attrs.k_before,
            k_after: attrs.k_after,
            delta_k_ratio: attrs.delta_k_ratio,
            fee_ppm: attrs.fee_ppm,
            input: attrs.input,
            output: attrs.output,
        },
    );
}

#[test]
fn remove_liquidity_span_exports_expected_attributes() {
    let attrs = RemoveLiquidityAttrs {
        k_before: 4.0,
        k_after: 3.8,
        delta_k_ratio: -0.05,
        fee_ppm: 80,
        input: 10.0,
        output: 9.5,
    };

    let spans = with_capturing_layer(|| {
        let span = span_amm_remove_liquidity(&attrs);
        let _guard = span.enter();
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "amm.remove_liquidity");
    expect_common_fields(
        span,
        "remove_liquidity",
        &SwapAttrs {
            k_before: attrs.k_before,
            k_after: attrs.k_after,
            delta_k_ratio: attrs.delta_k_ratio,
            fee_ppm: attrs.fee_ppm,
            input: attrs.input,
            output: attrs.output,
        },
    );
}

#[test]
fn remove_liquidity_wrapper_exports_expected_attributes() {
    let attrs = RemoveLiquidityAttrs {
        k_before: 4.5,
        k_after: 4.2,
        delta_k_ratio: -0.0666,
        fee_ppm: 75,
        input: 12.0,
        output: 11.2,
    };

    let spans = with_capturing_layer(|| {
        let result = in_amm_remove_liquidity(&attrs, || 11u32);
        assert_eq!(result, 11);
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "amm.remove_liquidity");
    expect_common_fields(
        span,
        "remove_liquidity",
        &SwapAttrs {
            k_before: attrs.k_before,
            k_after: attrs.k_after,
            delta_k_ratio: attrs.delta_k_ratio,
            fee_ppm: attrs.fee_ppm,
            input: attrs.input,
            output: attrs.output,
        },
    );
}

#[test]
fn pricing_quote_span_exports_expected_attributes() {
    let attrs = PricingQuoteAttrs {
        k_before: 7.0,
        k_after: 7.0,
        delta_k_ratio: 0.0,
        fee_ppm: 30,
        input: 200.0,
        output: 199.4,
    };

    let spans = with_capturing_layer(|| {
        let span = span_pricing_quote(&attrs);
        let _guard = span.enter();
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "pricing.quote");
    expect_common_fields(
        span,
        "pricing",
        &SwapAttrs {
            k_before: attrs.k_before,
            k_after: attrs.k_after,
            delta_k_ratio: attrs.delta_k_ratio,
            fee_ppm: attrs.fee_ppm,
            input: attrs.input,
            output: attrs.output,
        },
    );
}

#[test]
fn pricing_quote_wrapper_exports_expected_attributes() {
    let attrs = PricingQuoteAttrs {
        k_before: 6.0,
        k_after: 6.0,
        delta_k_ratio: 0.0,
        fee_ppm: 25,
        input: 80.0,
        output: 79.98,
    };

    let spans = with_capturing_layer(|| {
        let final_value = in_pricing_quote(&attrs, || 7);
        assert_eq!(final_value, 7);
    });

    assert_eq!(spans.len(), 1);
    let span = &spans[0];
    assert_eq!(span.name, "pricing.quote");
    expect_common_fields(
        span,
        "pricing",
        &SwapAttrs {
            k_before: attrs.k_before,
            k_after: attrs.k_after,
            delta_k_ratio: attrs.delta_k_ratio,
            fee_ppm: attrs.fee_ppm,
            input: attrs.input,
            output: attrs.output,
        },
    );
}

fn expect_panic<F>(f: F, message_contains: &str)
where
    F: FnOnce() + std::panic::UnwindSafe,
{
    let result = std::panic::catch_unwind(f);
    assert!(result.is_err(), "expected panic but function completed");
    let payload = result.err().unwrap();
    let panic_str = if let Some(msg) = payload.downcast_ref::<String>() {
        msg.as_str().to_owned()
    } else if let Some(msg) = payload.downcast_ref::<&'static str>() {
        msg.to_string()
    } else {
        format!("{:?}", payload)
    };
    assert!(
        panic_str.contains(message_contains),
        "panic message `{}` did not contain `{}`",
        panic_str,
        message_contains
    );
}

#[test]
fn invalid_attributes_trigger_panics() {
    let mut attrs = SwapAttrs {
        k_before: 1.0,
        k_after: 1.0,
        delta_k_ratio: 0.0,
        fee_ppm: 0,
        input: 0.0,
        output: 0.0,
    };

    attrs.k_before = f64::NAN;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "must be finite",
    );
    attrs.k_before = 1.0;

    attrs.k_after = 0.0;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "must be > 0",
    );
    attrs.k_after = 1.0;

    attrs.delta_k_ratio = f64::INFINITY;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "must be finite",
    );
    attrs.delta_k_ratio = 1_000_001.0;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "absolute value must be ≤ 1e6",
    );
    attrs.delta_k_ratio = 0.0;

    attrs.fee_ppm = -1;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "must be ≥ 0",
    );
    attrs.fee_ppm = 0;

    attrs.input = -0.1;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "must be ≥ 0",
    );
    attrs.input = 0.0;

    attrs.output = -0.1;
    expect_panic(
        || {
            span_amm_swap(&attrs);
        },
        "must be ≥ 0",
    );
}
