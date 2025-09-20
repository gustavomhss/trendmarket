use anyhow::Result;
use tracing::{info, span, Level};


#[tokio::main]
async fn main() -> Result<()> {
let handle = credit_engine_core::telemetry::init("credit-engine-core")?;
let span = span!(Level::INFO, "smoke_span", otel.name = "telemetry_smoke");
let _e = span.enter();
info!("emitindo métrica e finalizando…");
credit_engine_core::telemetry::bump_test_metric();
tokio::time::sleep(std::time::Duration::from_millis(200)).await;
handle.shutdown()?;
Ok(())
}
