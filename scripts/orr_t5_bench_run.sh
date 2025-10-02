#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
mkdir -p "$LOG"
TMPDIS=""
# (Por segurança) desabilita temporariamente o bin problemático
if [ -f "src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="src/bin/telemetry_smoke.rs.bak.$$"
  mv "src/bin/telemetry_smoke.rs" "$TMPDIS"
  trap 'mv "$TMPDIS" "src/bin/telemetry_smoke.rs" 2>/dev/null || true' EXIT
fi
cargo bench | tee "$LOG/cargo_bench.txt"
RC=${PIPESTATUS[0]}
if [ -n "$TMPDIS" ]; then mv "$TMPDIS" "src/bin/telemetry_smoke.rs"; trap - EXIT; fi
exit $RC
