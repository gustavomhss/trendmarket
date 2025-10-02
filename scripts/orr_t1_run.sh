#!/usr/bin/env bash
set -Eeuo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
mkdir -p "$LOG"

TMPDIS=""
restore_bin() {
  if [ -n "$TMPDIS" ] && [ -f "$TMPDIS" ]; then
    mv "$TMPDIS" "$ROOT/src/bin/telemetry_smoke.rs"
    TMPDIS=""
  fi
}

if [ -f "$ROOT/src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="$ROOT/src/bin/telemetry_smoke.rs.bak.$$"
  mv "$ROOT/src/bin/telemetry_smoke.rs" "$TMPDIS"
  trap 'restore_bin' EXIT INT TERM
fi

cd "$ROOT"
cargo test -- --nocapture | tee "$LOG/cargo_test_unit.txt"
RC=${PIPESTATUS[0]}
restore_bin
trap - EXIT INT TERM || true
exit $RC
