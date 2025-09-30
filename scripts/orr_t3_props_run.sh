#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"; EVI="$OUT/evidence/property"; DOC="$OUT/docs"
mkdir -p "$LOG" "$EVI" "$DOC" "$EVI/failures"

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG/t3_run.log"; }

note "Higiene: conflitos e placeholders"
if grep -RIn "^<<<<<<<\\|^=======\\|^>>>>>>>" -n . >/dev/null; then echo "ERRO: Conflitos detectados" | tee -a "$LOG/t3_run.log"; exit 2; fi
if grep -RInE '\\.\\.\\.|TBD|FIXME' -n tests/property >/dev/null 2>&1; then echo "ERRO: Placeholder em testes" | tee -a "$LOG/t3_run.log"; exit 3; fi

note "Rodando property tests (seed base fixa)"
export PROPTEST_CASES=${PROPTEST_CASES:-512}
export RUST_BACKTRACE=1
set -o pipefail
cargo test --tests -- --nocapture 2>&1 | tee "$LOG/cargo_test_property.txt"

note "Parse do log → summary.json + seeds.jsonl"
python3 - <<'PY'
import re, json, pathlib
root=pathlib.Path('out/orr_gatecheck')
log=(root/'logs'/'cargo_test_property.txt').read_text(encoding='utf-8', errors='ignore')
summary={"status":"UNKNOWN","passed":0,"failed":0,"ignored":0,"cases":int(__import__('os').getenv('PROPTEST_CASES','512'))}
# Heurística de contagem
status_counts = {"ok": 0, "FAILED": 0, "ignored": 0}
pattern = re.compile(r"test\s+(?:tests/property/[^\s]+|amm_[a-z_]+::[^\s]+)\s+...\s+(ok|FAILED|ignored)")
for match in pattern.finditer(log):
    status_counts[match.group(1)] += 1
summary["passed"] = status_counts["ok"]
summary["failed"] = status_counts["FAILED"]
summary["ignored"] = status_counts["ignored"]
summary["status"] = "GREEN" if summary["failed"]==0 else "RED"
(root/'evidence/property/summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
# Capturar seeds se o framework imprimir (depende do setup)
seeds=[]
for m in re.finditer(r"seed:\s*([0-9xA-Fa-f]+)", log):
    seeds.append({"seed":m.group(1)})
(root/'evidence/property/seeds.jsonl').write_text("\n".join(json.dumps(s) for s in seeds), encoding='utf-8')
print(json.dumps(summary, indent=2))
PY

note "Watchers finais"
! grep -RInE '\\.\\.\\.|TBD|FIXME' out/orr_gatecheck/docs || { echo "ERRO: placeholder na doc"; exit 4; }
! grep -RIn "^<<<<<<<\\|^=======\\|^>>>>>>>" -n . || { echo "ERRO: conflitos no repo"; exit 5; }

echo "OK: T3 finalizada"
