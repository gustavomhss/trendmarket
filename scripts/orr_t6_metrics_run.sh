#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
EVI="$OUT/evidence/metrics"
STEP="T6"

fail_read_only() {
  printf '{ "step":"%s", "error":"read_only" }\n' "$STEP"
  exit 95
}

require_write_access() {
  local target="$1"
  local dir="$target"
  if [ ! -d "$dir" ]; then
    dir="$(dirname "$dir")"
  fi
  while [ ! -d "$dir" ] && [ "$dir" != "/" ]; do
    dir="$(dirname "$dir")"
  done
  if [ ! -w "$dir" ]; then
    fail_read_only
  fi
  local probe
  if ! probe="$(mktemp "$dir/.writecheck.XXXXXX" 2>/dev/null)"; then
    fail_read_only
  fi
  rm -f "$probe"
}

require_write_access "$OUT"
require_write_access "$EVI"

mkdir -p "$EVI"

SMOKE_TMP=""
PORTS_TMP=""

cleanup() {
  if [ -n "$SMOKE_TMP" ] && [ -f "$SMOKE_TMP" ]; then
    rm -f "$SMOKE_TMP"
    SMOKE_TMP=""
  fi
  if [ -n "$PORTS_TMP" ] && [ -f "$PORTS_TMP" ]; then
    rm -f "$PORTS_TMP"
    PORTS_TMP=""
  fi
}

trap 'cleanup' EXIT INT TERM

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
python3 - "$SMOKE_TMP" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY
mv "$SMOKE_TMP" "$EVI/smoke.txt"
SMOKE_TMP=""

PORTS_TMP="$(mktemp "$EVI/ports.json.XXXXXX")"
cat >"$PORTS_TMP" <<'JSON'
{"http": 0, "grpc": 0}
JSON
python3 - "$PORTS_TMP" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY
mv "$PORTS_TMP" "$EVI/ports.json"
PORTS_TMP=""

cleanup
trap - EXIT INT TERM || true
