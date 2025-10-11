use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use tracing::{error, info, info_span};

#[path = "../obs4/mod.rs"]
mod obs4;

use obs4::tracing_init::init_tracing;

fn main() -> Result<()> {
    let guard = init_tracing().context("failed to initialize OpenTelemetry tracing")?;

    info_span!("pricing.quote", scenario = "smoke").in_scope(|| {
        info!(
            event = "pricing.quote.start",
            "simulating pricing quote span"
        );
        thread::sleep(Duration::from_millis(75));
        info!(event = "pricing.quote.complete", status = "ok");
    });

    info_span!("amm.swap", swap_id = 1_u64, mode = "ok").in_scope(|| {
        info!(event = "amm.swap.start", "beginning AMM swap span");
        thread::sleep(Duration::from_millis(90));
        info!(event = "amm.swap.complete", status = "ok", filled = 42);
    });

    let error_span = info_span!(
        "amm.swap",
        swap_id = 2_u64,
        mode = "error",
        otel.status_code = tracing::field::Empty,
        otel.status_description = tracing::field::Empty
    );

    {
        let _entered = error_span.enter();
        error_span.record("otel.status_code", &"ERROR");
        error_span.record("otel.status_description", &"slippage threshold breached");
        info!(event = "amm.swap.start", "starting AMM swap error span");
        thread::sleep(Duration::from_millis(120));
        error!(
            event = "amm.swap.failed",
            reason = "slippage limit exceeded"
        );
    }

    guard
        .shutdown()
        .context("failed to shutdown OpenTelemetry tracing")?;
    Ok(())
}
