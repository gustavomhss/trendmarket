use anyhow::Result;
use credit_engine_core::obs4::correlation::{current_trace_and_span, log_with_trace};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use serde_json::json;
use tracing::info_span;
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn init_tracing() -> Result<SdkTracerProvider> {
    let provider = SdkTracerProvider::builder().build();
    let tracer = provider.tracer("obs4_correlation_demo");

    let subscriber = Registry::default().with(tracing_opentelemetry::layer().with_tracer(tracer));

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(provider)
}

fn main() -> Result<()> {
    let provider = init_tracing()?;

    // Outside of any span, the helper still logs valid JSON but without trace correlation.
    log_with_trace(json!({
        "event": "startup",
        "detail": "no_span_context",
    }));

    let span = info_span!("amm.swap", otel.name = "amm.swap", op = "swap");
    let _guard = span.enter();

    log_with_trace(json!({
        "event": "swap_ok",
        "latency_ms": 12.4,
    }));

    if let Some((trace_id, span_id)) = current_trace_and_span() {
        log_with_trace(json!({
            "event": "trace_debug",
            "trace_id_echo": trace_id,
            "span_id_echo": span_id,
        }));
    }

    drop(_guard);
    drop(span);

    let _ = provider.force_flush();
    let _ = provider.shutdown();
    Ok(())
}
