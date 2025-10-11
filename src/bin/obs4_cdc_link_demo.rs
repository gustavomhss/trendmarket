use anyhow::Result;
use credit_engine_core::obs4::propagation::{extract_map, inject_map, link_from_cdc};
use opentelemetry::{
    global,
    trace::{Span, TraceContextExt, Tracer, TracerProvider as _},
    Context,
};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};
use std::collections::BTreeMap;

fn main() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = SdkTracerProvider::builder().build();
    let tracer = provider.tracer("obs4_cdc_link_demo");
    let _ = global::set_tracer_provider(provider.clone());

    let cdc_span = tracer.start("cdc.consume");
    let cdc_context = Context::current_with_span(cdc_span);
    let _cdc_guard = cdc_context.clone().attach();

    let mut headers = BTreeMap::new();
    inject_map(&mut headers);
    let produced = headers
        .get("traceparent")
        .cloned()
        .unwrap_or_else(|| "".to_string());
    println!("cdc emitted traceparent={produced}");

    drop(_cdc_guard);
    cdc_context.span().end();

    let consumer_context = extract_map(&headers);
    let cdc_span_context = consumer_context.span().span_context().clone();
    let link = link_from_cdc(&cdc_span_context);

    let mut amm_span = tracer.start("amm.swap");
    amm_span.add_link(link.span_context.clone(), link.attributes.clone());
    let amm_trace = amm_span.span_context().trace_id().to_string();
    println!(
        "amm.swap trace_id={amm_trace} linked_cdc_trace_id={}",
        cdc_span_context.trace_id()
    );
    amm_span.end();

    provider.shutdown().ok();
    Ok(())
}
