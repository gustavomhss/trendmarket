#!/usr/bin/env python3
"""Validador T8 — Consolida evidências T1..T7 e emite resumo final do ORR.
Saída: out/orr_gatecheck/evidence/orr_final_summary.json
"""
import json, pathlib, re, sys
from pathlib import Path
ROOT=Path('.')
OUT=ROOT/'out'/'orr_gatecheck'
EVI=OUT/'evidence'
DOC=OUT/'docs'
EVI.mkdir(parents=True, exist_ok=True)
DOC.mkdir(parents=True, exist_ok=True)

# Util
p=lambda *a: ROOT.joinpath(*a)
rd=lambda pth: json.loads(Path(pth).read_text(encoding='utf-8')) if Path(pth).exists() else None

# Coleta
static     = rd(EVI/'orr_static_summary.json')
unit       = rd(EVI/'unit'/'summary.json')
props      = rd(EVI/'property'/'summary.json')
goldens    = rd(EVI/'goldens'/'summary.json')
bench_base = rd(EVI/'bench'/'baseline'/'criterion_summary.json')
bench_dlt  = rd(EVI/'bench'/'delta.json')
metrics_ok = (EVI/'metrics'/'smoke.txt').exists() and (EVI/'metrics'/'ports.json').exists()
ci_run     = rd(EVI/'ci'/'run_summary.json')

# Status helpers
def ci_green(run_summary):
  """Return True when the CI evidence shows a completed successful run."""
  if isinstance(run_summary, dict):
    runs = [run_summary]
  elif isinstance(run_summary, list):
    runs = run_summary
  else:
    return False

  for run in runs:
    if not isinstance(run, dict):
      continue
    status = str(run.get('status', '')).lower()
    conclusion = str(run.get('conclusion', '')).lower()
    if status == 'completed' and conclusion == 'success':
      return True
  return False

status = {
  'unit':    'GREEN' if unit and unit.get('status')=='GREEN' and unit.get('failed',1)==0 else 'RED',
  'property':'GREEN' if props and props.get('status')=='GREEN' and props.get('failed',1)==0 else 'RED',
  'goldens': 'GREEN' if goldens and goldens.get('status')=='GREEN' and goldens.get('mismatch',1)==0 else 'RED',
  'bench':   'GREEN' if bench_base and (not bench_dlt or bench_dlt.get('status','GREEN')=='GREEN') else 'RED',
  'metrics': 'GREEN' if metrics_ok else 'RED',
  'ci':      'GREEN' if ci_green(ci_run) else 'RED',
}
kill = sum(1 for v in status.values() if v!='GREEN')
overall = 'GREEN' if kill==0 else 'RED'

# Checklist de links
checklist = {
  'unit': [ 'out/orr_gatecheck/logs/cargo_test_unit.txt', 'out/orr_gatecheck/evidence/unit/summary.json', 'out/orr_gatecheck/docs/ORR_UNIT.md' ],
  'property': [ 'out/orr_gatecheck/logs/cargo_test_property.txt', 'out/orr_gatecheck/evidence/property/summary.json', 'out/orr_gatecheck/docs/ORR_PROPERTY.md' ],
  'goldens': [ 'out/orr_gatecheck/logs/cargo_test_goldens.txt', 'out/orr_gatecheck/evidence/goldens/summary.json', 'out/orr_gatecheck/evidence/goldens/diff_reports/' ],
  'bench': [ 'out/orr_gatecheck/logs/cargo_bench.txt', 'out/orr_gatecheck/evidence/bench/baseline/criterion_summary.json', 'out/orr_gatecheck/evidence/bench/delta.json', 'out/orr_gatecheck/docs/ORR_BENCH.md' ],
  'metrics': [ 'out/orr_gatecheck/evidence/metrics/smoke.txt', 'out/orr_gatecheck/evidence/metrics/ports.json', 'out/orr_gatecheck/docs/ORR_METRICS.md' ],
  'ci': [ '.github/workflows/ci.yml', 'out/orr_gatecheck/evidence/ci/run_summary.json', 'out/orr_gatecheck/docs/ORR_CI.md' ],
}

summary={
  'timestamp': __import__('datetime').datetime.now().isoformat(),
  'exits': status,
  'kill_criteria_count': kill,
  'overall': overall,
  'checklist_links': checklist,
}
(EVI/'orr_final_summary.json').write_text(json.dumps(summary, indent=2, ensure_ascii=False), encoding='utf-8')
print(json.dumps({'overall': overall, 'kill': kill}, indent=2))

# Falhar se RED (para o driver decidir)
if overall!='GREEN':
  sys.exit(3)
