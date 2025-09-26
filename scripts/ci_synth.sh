#!/usr/bin/env bash
set -euo pipefail


./scripts/otel_reset_stack_0300.sh
./scripts/otel_assert_versions.sh
./scripts/obs_feature_gate_fix.sh


echo "[ci] check sem feature obs…"
cargo check -q


echo "[ci] check com feature obs…"
cargo check -q --features obs


./scripts/dev_stack_up.sh


# Smoke (usa bin criado anteriormente)
if [ -f scripts/telemetry_smoke.sh ]; then
./scripts/telemetry_smoke.sh
else
echo "[ci] criando smoke runner…"
mkdir -p src/bin
cat > src/bin/telemetry_smoke.rs <<'RS'
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
RS
RUST_LOG=${RUST_LOG:-info} \
OTEL_EXPORTER_OTLP_ENDPOINT=${OTEL_EXPORTER_OTLP_ENDPOINT:-http://localhost:4318} \
cargo run --bin telemetry_smoke -q
fi


echo "\n✅ SUCESSO TOTAL — stack alinhado, build OK, trace/metric emitidos."
