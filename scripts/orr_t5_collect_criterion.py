#!/usr/bin/env python3
import json, sys
from pathlib import Path
ROOT = Path('.')
OUT = ROOT/'out'/'orr_gatecheck'
EVI = OUT/'evidence'/'bench'/'baseline'
EVI.mkdir(parents=True, exist_ok=True)
paths = list(ROOT.glob('target/criterion/**/new/estimates.json'))
summary = []
for p in paths:
    try:
        data = json.loads(p.read_text(encoding='utf-8'))
    except Exception:
        continue
    summary.append({
        'benchmark': str(p.parent.parent.parent.name),
        'mean_point_estimate': data.get('mean',{}).get('point_estimate'),
        'median_point_estimate': data.get('median',{}).get('point_estimate'),
        'slope_point_estimate': data.get('slope',{}).get('point_estimate')
    })
out = {'count': len(summary), 'benchmarks': summary}
(EVI/'criterion_summary.json').write_text(json.dumps(out, ensure_ascii=False, indent=2), encoding='utf-8')
if out['count'] == 0:
    sys.stderr.write('No Criterion artifacts found; run benches then collect.\n')
    sys.exit(3)
print(json.dumps({'count': out['count']}))
