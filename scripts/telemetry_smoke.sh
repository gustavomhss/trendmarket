#!/usr/bin/env bash
set -euo pipefail
export RUST_LOG=${RUST_LOG:-info}
[ -f ./.env ] && set -a && . ./.env && set +a


./scripts/otel_preflight_endpoint.sh || exit 2


set +e
cargo run --bin telemetry_smoke -q
code=$?
set -e


if [ $code -ne 0 ]; then
if [[ "${TELEMETRY_SHUTDOWN_TOLERANT:-}" == "1" ]]; then
echo "[smoke] ⚠️ erro durante shutdown de telemetry (tolerado)."; exit 0
fi
echo "[smoke] ❌ falhou (código $code). Logs do collector:";
docker logs otel-collector --tail=200 2>/dev/null || true
exit $code
fi


echo "[smoke] ✅ OK — cheque Jaeger e Prometheus (URLs acima)."
