use std::fmt;

use credit_engine_core::obs4::spans::{
    set_status_from_result, span_amm_swap, GuardrailEvent, SwapReq,
};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_sdk::trace::SdkTracerProvider;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, Registry};

fn main() {
    let provider = init_tracing();

    run_demo();

    shutdown(provider);
}

fn init_tracing() -> SdkTracerProvider {
    let provider = SdkTracerProvider::builder().build();
    let tracer = provider.tracer("obs4_span_demo");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_timer(tracing_subscriber::fmt::time::uptime());

    let subscriber = Registry::default().with(otel_layer).with(fmt_layer);
    tracing::subscriber::set_global_default(subscriber).expect("install tracing subscriber");
    let _ = global::set_tracer_provider(provider.clone());
    provider
}

fn run_demo() {
    info!("obs4_span_demo starting");

    let ok_req = SwapReq {
        k_before: 1_500_000.0,
        k_after: 1_505_500.0,
        delta_k_ratio: 0.0036,
        fee_ppm: 300,
        input_amount: 2_500.0,
        output_amount: 2_492.4,
        asset_in: "USDC-POOL-PRIMARY",
        asset_out: "BRL-POOL-2024Q4",
        guardrail_hit: false,
        guardrail: None,
        rounding_rule: Some("bankers_rounding"),
    };

    let ok_span = span_amm_swap(&ok_req);
    {
        let _guard = ok_span.enter();
        info!("executing happy-path swap");
    }
    let ok_result: Result<(), DemoError> = Ok(());
    set_status_from_result(&ok_span, &ok_result);
    drop(ok_span);

    let err_req = SwapReq {
        k_before: 1_505_500.0,
        k_after: 1_505_500.0,
        delta_k_ratio: 0.0,
        fee_ppm: 450,
        input_amount: 10_000.0,
        output_amount: 0.0,
        asset_in: "USDC-POOL-PRIMARY",
        asset_out: "BRL-POOL-2024Q4",
        guardrail_hit: true,
        guardrail: Some(GuardrailEvent {
            code: "CE-AMM-GRD-429",
            reason: "liquidity cap breached",
        }),
        rounding_rule: None,
    };

    let err_span = span_amm_swap(&err_req);
    {
        let _guard = err_span.enter();
        error!("swap rejected by guardrail");
    }
    let err_result: Result<(), DemoError> = Err(DemoError("guardrail rejection"));
    set_status_from_result(&err_span, &err_result);
    drop(err_span);

    info!("obs4_span_demo completed");
}

fn shutdown(provider: SdkTracerProvider) {
    if let Err(err) = provider.shutdown() {
        eprintln!("obs4_span_demo: exporter shutdown error: {err}");
    }
}

#[derive(Debug)]
struct DemoError(&'static str);

impl fmt::Display for DemoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DemoError {}
