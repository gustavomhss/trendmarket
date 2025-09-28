use anyhow::Result;
use opentelemetry::KeyValue;
use std::time::Instant;

use credit_engine_core::amm::errors::AmmError;
use credit_engine_core::telemetry; // troque para ce_core se o crate tiver esse nome

#[tokio::main]
async fn main() -> Result<()> {
    let tel = telemetry::init("credit-engine-core")?;

    for i in 0..5u32 {
        let span = telemetry::make_info_span("swap", i, "obs_demo");
        let _guard = span.enter();

        let t0 = Instant::now();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;

        tel.swap_latency_ms
            .record(elapsed_ms, &[KeyValue::new("op", "swap")]);
        tel.invariant_error_rel
            .record(0.001_f64, &[KeyValue::new("op", "swap")]);
    }

    // Emit an example error log with the hardened contract fields.
    let sample_error = AmmError::MinReserveBreached;
    if let Some(status) = sample_error.http_status() {
        tracing::error!(
            error_code = sample_error.error_code(),
            error_message = sample_error.user_message(),
            error_variant = sample_error.variant_name(),
            http_status = status,
            "sample AMM error emitted"
        );
    } else {
        tracing::error!(
            error_code = sample_error.error_code(),
            error_message = sample_error.user_message(),
            error_variant = sample_error.variant_name(),
            "sample AMM error emitted"
        );
    }

    tel.shutdown();
    Ok(())
}
