#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
EVIDENCE_BASE="$OUT/evidence"
UNIT_EVIDENCE="$EVIDENCE_BASE/unit"
DOCS="$OUT/docs"
mkdir -p "$LOG" "$EVIDENCE_BASE" "$UNIT_EVIDENCE" "$DOCS"
TMPDIS=""
if [ -f "src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="src/bin/telemetry_smoke.rs.bak.$$"
  mv "src/bin/telemetry_smoke.rs" "$TMPDIS"
  trap 'mv "$TMPDIS" "src/bin/telemetry_smoke.rs" 2>/dev/null || true' EXIT
fi
cargo test -- --nocapture | tee "$LOG/cargo_test_unit.txt"
RC=${PIPESTATUS[0]}
if [ -n "$TMPDIS" ]; then mv "$TMPDIS" "src/bin/telemetry_smoke.rs"; trap - EXIT; fi
exit $RC
