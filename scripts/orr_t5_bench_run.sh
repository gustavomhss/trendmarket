#!/usr/bin/env bash
set -Eeuo pipefail
export LC_ALL=C
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"; BASE="$OUT/evidence/bench/baseline"; DOC="$OUT/docs"
mkdir -p "$LOG" "$BASE" "$DOC"

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG/t5_run.log"; }

note "Higiene: conflitos e placeholders"
if grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . >/dev/null; then echo "ERRO: Conflitos detectados" | tee -a "$LOG/t5_run.log"; exit 2; fi
if grep -RInE '\\.\\.\\.|TBD|FIXME' -n benches bench >/dev/null 2>&1; then echo "ERRO: Placeholder em benches" | tee -a "$LOG/t5_run.log"; exit 3; fi

note "Executando cargo bench (criterion)"
export CRITERION_SAMPLE_SIZE=${CRITERION_SAMPLE_SIZE:-100}
export CRITERION_MEASUREMENT_TIME=${CRITERION_MEASUREMENT_TIME:-3}
set -o pipefail
if ! cargo bench 2>&1 | tee "$LOG/cargo_bench.txt"; then
  note "cargo bench falhou"
  exit 1
fi

note "Coletando estimates.json"
python3 scripts/orr_t5_collect_criterion.py | tee -a "$LOG/t5_collect.txt"

note "Comparando com baseline aprovado (se existir)"
THRESHOLD=${THRESHOLD_REGRESSION_PCT:-10}
python3 - <<PY 2>&1 | tee -a "$LOG/t5_delta.txt"
import json, pathlib, os, sys
out=pathlib.Path('out/orr_gatecheck')
base=out/'evidence/bench/baseline/criterion_summary.json'
approv=pathlib.Path('benchmarks/baseline/approved.json')
res={'status':'GREEN','threshold_pct':int(os.getenv('THRESHOLD_REGRESSION_PCT','10')),'deltas':[]}
if approv.exists() and base.exists():
    A=json.loads(approv.read_text())
    B=json.loads(base.read_text())
    a=dict((i['id'], i) for i in A.get('entries',[]))
    for b in B.get('entries',[]):
        bid=b['id']
        if bid in a and 'mean_ns' in a[bid] and 'mean_ns' in b:
            old=a[bid]['mean_ns']; new=b['mean_ns']
            if old>0:
                pct=(new-old)/old*100.0
                res['deltas'].append({'id':bid,'old_mean_ns':old,'new_mean_ns':new,'delta_pct':pct})
    # status
    for d in res['deltas']:
        if d['delta_pct']>res['threshold_pct']:
            res['status']='RED'
            break
(out/'evidence/bench/delta.json').write_text(json.dumps(res, indent=2), encoding='utf-8')
print(json.dumps(res, indent=2))
PY

if grep -q '"status": "RED"' "$OUT/evidence/bench/delta.json" 2>/dev/null; then
  echo "REGRESSÃO acima do limiar" | tee -a "$LOG/t5_run.log"
  exit 4
fi

note "T5 concluída"
