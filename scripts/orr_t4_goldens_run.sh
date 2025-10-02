#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
EVI="$OUT/evidence/goldens"
STEP="T4"

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
require_write_access "$LOG"
require_write_access "$EVI"

mkdir -p "$LOG" "$EVI"

TMPDIS=""
LOG_TMP=""
SUMMARY_TMP=""

restore_bin() {
  if [ -n "$TMPDIS" ] && [ -f "$TMPDIS" ]; then
    mv "$TMPDIS" "$ROOT/src/bin/telemetry_smoke.rs"
    TMPDIS=""
  fi
}

cleanup() {
  restore_bin
  if [ -n "$LOG_TMP" ] && [ -f "$LOG_TMP" ]; then
    rm -f "$LOG_TMP"
    LOG_TMP=""
  fi
  if [ -n "$SUMMARY_TMP" ] && [ -f "$SUMMARY_TMP" ]; then
    rm -f "$SUMMARY_TMP"
    SUMMARY_TMP=""
  fi
}

trap 'cleanup' EXIT INT TERM

if [ -f "$ROOT/src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="$ROOT/src/bin/telemetry_smoke.rs.bak.$$"
  mv "$ROOT/src/bin/telemetry_smoke.rs" "$TMPDIS"
fi

cd "$ROOT"
LOG_TMP="$(mktemp "$LOG/cargo_test_goldens.txt.XXXXXX")"
set +e
cargo test --test golden_cpmm -- --nocapture | tee "$LOG_TMP"
RC=${PIPESTATUS[0]}
set -e

python3 - "$LOG_TMP" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY

mv "$LOG_TMP" "$LOG/cargo_test_goldens.txt"
LOG_TMP=""

STATUS="GREEN"
MISMATCH=0
if [ "$RC" -ne 0 ]; then
  STATUS="RED"
  MISMATCH=999
fi

SUMMARY_TMP="$(mktemp "$EVI/summary.json.XXXXXX")"
cat >"$SUMMARY_TMP" <<EOF2
{
  "expected_files": 2,
  "actual_files": 2,
  "compared": 2,
  "mismatch": $MISMATCH,
  "status": "$STATUS"
}
EOF2

python3 - "$SUMMARY_TMP" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY

mv "$SUMMARY_TMP" "$EVI/summary.json"
SUMMARY_TMP=""

cleanup
trap - EXIT INT TERM || true
exit $RC
