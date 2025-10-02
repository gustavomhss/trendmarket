#!/usr/bin/env python3
import json
import os
import sys
import tempfile
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
OUT = ROOT / 'out' / 'orr_gatecheck'
EVI = OUT / 'evidence' / 'bench' / 'baseline'
EVI.mkdir(parents=True, exist_ok=True)

paths = sorted(ROOT.glob('target/criterion/**/new/estimates.json'))
benchmarks = []
for path in paths:
    try:
        data = json.loads(path.read_text(encoding='utf-8'))
    except Exception:
        continue
    parents = path.parents
    bench_name = parents[1].name if len(parents) > 1 else path.parent.name
    benchmarks.append({
        'benchmark': bench_name,
        'mean_point_estimate': data.get('mean', {}).get('point_estimate'),
        'median_point_estimate': data.get('median', {}).get('point_estimate'),
        'slope_point_estimate': data.get('slope', {}).get('point_estimate'),
    })

output = {
    'count': len(benchmarks),
    'benchmarks': benchmarks,
}

target = EVI / 'criterion_summary.json'
with tempfile.NamedTemporaryFile('w', encoding='utf-8', delete=False, dir=str(EVI), prefix='criterion_summary.', suffix='.json') as tmp:
    json.dump(output, tmp, indent=2)
    tmp.flush()
    os.fsync(tmp.fileno())
temp_path = Path(tmp.name)
temp_path.replace(target)

if output['count'] == 0:
    sys.stderr.write('No Criterion artifacts found; run benches then collect.\n')
    sys.exit(3)

print(json.dumps({'count': output['count']}))
