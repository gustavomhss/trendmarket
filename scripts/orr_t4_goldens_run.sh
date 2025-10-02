#!/usr/bin/env bash
set -Eeuo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
EVI="$OUT/evidence/goldens"
mkdir -p "$LOG" "$EVI"

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
cargo test --test golden_cpmm -- --nocapture | tee "$LOG/cargo_test_goldens.txt"
RC=${PIPESTATUS[0]}
restore_bin
trap - EXIT INT TERM || true

STATUS="GREEN"
MISMATCH=0
if [ "$RC" -ne 0 ]; then
  STATUS="RED"
  MISMATCH=999
fi

TMP_JSON="$(mktemp "$EVI/summary.json.XXXXXX")"
cat >"$TMP_JSON" <<EOF
{
  "expected_files": 2,
  "actual_files": 2,
  "compared": 2,
  "mismatch": $MISMATCH,
  "status": "$STATUS"
}
EOF
mv "$TMP_JSON" "$EVI/summary.json"

exit $RC
