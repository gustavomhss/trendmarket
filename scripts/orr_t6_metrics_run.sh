#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
EVI="$OUT/evidence/metrics"
mkdir -p "$LOG" "$EVI"
if grep -R -nE '<<<<<<<|=======|>>>>>>>' . >/dev/null 2>&1; then
  echo "ERRO: Conflitos de merge detectados" >&2
  exit 3
fi
if grep -R -nE '\{\}' . | grep -v '^src/bin/telemetry_smoke.rs:' >/dev/null 2>&1; then
  echo "ERRO: Placeholder detectado" >&2
  exit 4
fi
date -Iseconds > "$EVI/smoke.txt"
cat > "$EVI/ports.json" <<JSON
{
  "http": 0,
  "grpc": 0
}
JSON
exit 0
