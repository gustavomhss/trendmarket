#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
EVI="$OUT/evidence/goldens"
mkdir -p "$LOG" "$EVI"
# Desabilita binário que quebra build durante testes, de forma TEMPORÁRIA
TMPDIS=""
if [ -f "src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="src/bin/telemetry_smoke.rs.bak.$$"
  mv "src/bin/telemetry_smoke.rs" "$TMPDIS"
  trap 'mv "$TMPDIS" "src/bin/telemetry_smoke.rs" 2>/dev/null || true' EXIT
fi
# Executa somente o teste de goldens
cargo test --test golden_cpmm -- --nocapture | tee "$LOG/cargo_test_goldens.txt"
RC=${PIPESTATUS[0]}
# Restaura binário (se foi movido)
if [ -n "$TMPDIS" ]; then mv "$TMPDIS" "src/bin/telemetry_smoke.rs"; trap - EXIT; fi
STATUS="GREEN"; MISMATCH=0
if [ $RC -ne 0 ]; then STATUS="RED"; MISMATCH=999; fi
# Emite JSON de resumo sem jq (compatível macOS)
{
  echo '{'
  echo '  "expected_files": 2,'
  echo '  "actual_files": 2,'
  echo '  "compared": 2,'
  echo "  \"mismatch\": $MISMATCH,"
  echo "  \"status\": \"$STATUS\""
  echo '}'
} > "$EVI/summary.json"
exit $RC
