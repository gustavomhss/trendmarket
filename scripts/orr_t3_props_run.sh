#!/usr/bin/env bash
set -Eeuo pipefail


# Descobre a raiz do repo de forma robusta
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"
EVI="$OUT/evidence/property"
mkdir -p "$LOG" "$EVI"


# Desabilita bin problemático durante testes (reabilita ao final)
TMPDIS=""
if [ -f "$ROOT/src/bin/telemetry_smoke.rs" ]; then
TMPDIS="$ROOT/src/bin/telemetry_smoke.rs.bak.$$"
mv "$ROOT/src/bin/telemetry_smoke.rs" "$TMPDIS"
trap 'mv "$TMPDIS" "$ROOT/src/bin/telemetry_smoke.rs" 2>/dev/null || true' EXIT
fi


# Executa a suíte de propriedades
cargo test --test property -- --nocapture | tee "$LOG/cargo_test_property.txt"
RC=${PIPESTATUS[0]}


# Reverte o bin (se foi movido)
if [ -n "${TMPDIS:-}" ]; then mv "$TMPDIS" "$ROOT/src/bin/telemetry_smoke.rs"; trap - EXIT; fi


# Extrai seeds (se houver)
SEEDS_TMP="$(mktemp)"
grep -aE '^seed:[0-9]+' "$LOG/cargo_test_property.txt" > "$SEEDS_TMP" || true
cp "$SEEDS_TMP" "$EVI/seeds.jsonl" 2>/dev/null || true
rm -f "$SEEDS_TMP"


# Conta pass/fail a partir do log
PASS=$(grep -aE '^test .* \.\.\. ok$' "$LOG/cargo_test_property.txt" | wc -l | tr -d ' ')
FAIL=$(grep -aE '^test .* \.\.\. FAILED$' "$LOG/cargo_test_property.txt" | wc -l | tr -d ' ')
STATUS="GREEN"; if [ "$RC" -ne 0 ] || [ "${FAIL:-0}" -gt 0 ]; then STATUS="RED"; fi


# Escreve summary de forma atômica
TMPJSON="$(mktemp)"
printf '{\n "status":"%s",\n "passed":%s,\n "failed":%s\n}\n' "$STATUS" "$PASS" "$FAIL" > "$TMPJSON"
mv "$TMPJSON" "$EVI/summary.json"


exit "$RC"
