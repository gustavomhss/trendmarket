#!/usr/bin/env bash
set -euo pipefail
OUT=$(cargo tree -d | grep -E "opentelemetry(_sdk|-otlp|-http|-proto)?|tracing-opentelemetry" || true)
if [ -n "$OUT" ]; then
echo "$OUT"
echo "\n❌ Ainda existem duplicatas de OpenTelemetry. Rode: scripts/telemetry_hunt_dups.sh"; exit 1
else
echo "✅ Sem duplicatas de OpenTelemetry/Tracing"
fi
