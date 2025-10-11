use anyhow::Result;
use credit_engine_core::obs4::propagation::{extract_parent, inject_headers};
use http::Request;
use opentelemetry::{
    global,
    trace::{Span, TraceContextExt, Tracer, TracerProvider as _},
    Context,
};
use opentelemetry_sdk::{propagation::TraceContextPropagator, trace::SdkTracerProvider};

fn main() -> Result<()> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let provider = SdkTracerProvider::builder().build();
    let tracer = provider.tracer("obs4_http_demo");
    let _ = global::set_tracer_provider(provider.clone());

    let client_span = tracer.start("http.client");
    let client_context = Context::current_with_span(client_span);
    let _client_guard = client_context.clone().attach();

    let mut request_builder = Request::builder()
        .method("GET")
        .uri("https://example.com/price");
    inject_headers(&mut request_builder);
    let request = request_builder.body(())?;

    let traceparent = request
        .headers()
        .get("traceparent")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    println!("client propagated traceparent={traceparent}");

    drop(_client_guard);
    client_context.span().end();

    let parent_context = extract_parent(request.headers());
    let mut server_span = tracer.start_with_context("http.server", &parent_context);
    let server_trace_id = server_span.span_context().trace_id().to_string();
    println!("server child span trace_id={server_trace_id}");
    server_span.end();

    provider.shutdown().ok();
    Ok(())
}
