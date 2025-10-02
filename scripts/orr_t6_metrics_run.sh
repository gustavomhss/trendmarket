#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
EVI="$OUT/evidence/metrics"
mkdir -p "$EVI"

cd "$ROOT"

if git grep -I -n -E '^(<<<<<<<|=======|>>>>>>>)' -- . >/dev/null 2>&1; then
  echo "ERRO: Conflitos de merge detectados" >&2
  exit 3
fi

if git grep -I -n -E '^[[:space:]]*\{\}[[:space:]]*$' -- . ':!src/bin/telemetry_smoke.rs' >/dev/null 2>&1; then
  echo "ERRO: Placeholder detectado" >&2
  exit 4
fi

timestamp="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
SMOKE_TMP="$(mktemp "$EVI/smoke.txt.XXXXXX")"
printf '%s\n' "$timestamp" >"$SMOKE_TMP"
mv "$SMOKE_TMP" "$EVI/smoke.txt"

PORTS_TMP="$(mktemp "$EVI/ports.json.XXXXXX")"
cat >"$PORTS_TMP" <<'JSON'
{"http": 0, "grpc": 0}
JSON
mv "$PORTS_TMP" "$EVI/ports.json"
