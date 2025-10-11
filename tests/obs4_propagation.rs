use credit_engine_core::obs4::propagation::{
    extract_map, extract_parent, inject_headers, inject_map, link_from_cdc,
};
use http::Request;
use opentelemetry::{
    global,
    testing::trace::TestSpan,
    trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
    Context,
};
use opentelemetry_sdk::propagation::TraceContextPropagator;
use std::collections::BTreeMap;
use std::sync::Once;

static INIT: Once = Once::new();

fn setup_propagator() {
    INIT.call_once(|| {
        global::set_text_map_propagator(TraceContextPropagator::new());
    });
}

fn build_test_context(trace_id: TraceId) -> (Context, SpanContext) {
    let span_id = SpanId::from_hex("0102030405060708").expect("span id");
    let span_context = SpanContext::new(
        trace_id,
        span_id,
        TraceFlags::SAMPLED,
        false,
        TraceState::default(),
    );
    let span = TestSpan(span_context.clone());
    let ctx = Context::current_with_span(span);
    (ctx, span_context)
}

#[test]
fn http_roundtrip_preserves_trace_id() {
    setup_propagator();

    let trace_id = TraceId::from_hex("00112233445566778899aabbccddeeff").expect("trace id");
    let (ctx, original_span_context) = build_test_context(trace_id);
    let _guard = ctx.clone().attach();

    let mut builder = Request::builder();
    inject_headers(&mut builder);
    let request = builder.body(()).expect("request body");

    drop(_guard);

    let extracted = extract_parent(request.headers());
    let extracted_ctx = extracted.span().span_context().clone();

    assert_eq!(extracted_ctx.trace_id(), original_span_context.trace_id());
}

#[test]
fn map_roundtrip_preserves_trace_id() {
    setup_propagator();

    let trace_id = TraceId::from_hex("f0e1d2c3b4a5968778695a4b3c2d1e0f").expect("trace id");
    let (ctx, original_span_context) = build_test_context(trace_id);
    let _guard = ctx.clone().attach();

    let mut headers = BTreeMap::new();
    inject_map(&mut headers);

    drop(_guard);

    let extracted = extract_map(&headers);
    let extracted_ctx = extracted.span().span_context().clone();

    assert_eq!(extracted_ctx.trace_id(), original_span_context.trace_id());
}

#[test]
fn link_uses_cdc_trace_id() {
    let trace_id = TraceId::from_hex("1234567890abcdef1234567890abcdef").expect("trace id");
    let span_context = SpanContext::new(
        trace_id,
        SpanId::from_hex("deafbeefcafebabe").expect("span id"),
        TraceFlags::SAMPLED,
        true,
        TraceState::default(),
    );

    let link = link_from_cdc(&span_context);
    assert_eq!(link.span_context.trace_id(), span_context.trace_id());
}
