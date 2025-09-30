#!/usr/bin/env python3
"""Coleta estimates.json do Criterion e consolida baseline.
Saída: out/orr_gatecheck/evidence/bench/baseline/criterion_summary.json
"""
import json, pathlib
from pathlib import Path
root = Path('.')
crit = root/'target'/'criterion'
entries=[]
for est in crit.rglob('estimates.json'):
    j=json.loads(est.read_text())
    bench_id=str(est.parent.relative_to(crit))  # ex.: group/bench
    mean=j.get('mean',{}).get('point_estimate')
    median=j.get('median',{}).get('point_estimate')
    slope=j.get('slope',{}).get('point_estimate')
    entries.append({
        'id': bench_id,
        'mean_ns': mean,
        'median_ns': median,
        'slope': slope,
        'path': str(est)
    })
out = root/'out'/'orr_gatecheck'/'evidence'/'bench'/'baseline'/'criterion_summary.json'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps({'entries':entries}, indent=2), encoding='utf-8')
print(json.dumps({'count':len(entries)}, indent=2))
