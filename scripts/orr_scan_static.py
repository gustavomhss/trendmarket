#!/usr/bin/env python3
"""Scanner estático do ORR (T1)
- Detecta exits: unit, property, goldens, bench, métricas, CI
- Produz JSON em out/orr_gatecheck/evidence/orr_static_summary.json
- NÃO altera código de produto
"""
import re, json, sys
from pathlib import Path

ROOT = Path('.').resolve()
OUT = ROOT / 'out' / 'orr_gatecheck' / 'evidence'
OUT.mkdir(parents=True, exist_ok=True)

# Heurísticas de detecção
UNIT_SIGNS = [r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", r"mod\s+tests\s*\{"]
PROP_SIGNS = [r"proptest!", r"proptest::", r"quickcheck"]
GOLDEN_DIRS = [ROOT/"tests"/"goldens", ROOT/"goldens"]
BENCH_DIRS  = [ROOT/"bench", ROOT/"benches"]
METRICS_SIGNS = ["opentelemetry", "prometheus", "metrics::"]
CI_DIR = ROOT/".github"/"workflows"

# Util
def rel(p: Path) -> str:
    try:
        return str(p.relative_to(ROOT))
    except Exception:
        return str(p)

rs_files = [p for p in ROOT.rglob('*.rs') if 'target' not in p.parts]

unit = []
for p in rs_files:
    tx = p.read_text(encoding='utf-8', errors='ignore')
    if any(re.search(sig, tx) for sig in UNIT_SIGNS):
        unit.append(rel(p))

property_tests = []
for p in rs_files:
    tx = p.read_text(encoding='utf-8', errors='ignore').lower()
    if any(sig in tx for sig in [s.lower() for s in PROP_SIGNS]):
        property_tests.append(rel(p))

golden_tests = []
for d in GOLDEN_DIRS:
    if d.exists():
        golden_tests += [rel(p) for p in d.rglob('*.rs')]

golden_assets = []
if (ROOT/"goldens").exists():
    golden_assets = [rel(p) for p in (ROOT/"goldens").rglob('*') if p.is_file()]

benches = []
for d in BENCH_DIRS:
    if d.exists():
        benches += [rel(p) for p in d.rglob('*.rs')]

metrics_hits = []
for p in rs_files:
    tx = p.read_text(encoding='utf-8', errors='ignore').lower()
    if any(k in tx for k in [s.lower() for s in METRICS_SIGNS]):
        metrics_hits.append(rel(p))

ci_workflows = []
if CI_DIR.exists():
    ci_workflows = [rel(p) for p in CI_DIR.glob('*') if p.is_file()]

exits = {
    'unit': 'GREEN' if unit else 'RED',
    'property': 'GREEN' if property_tests else 'RED',
    'goldens': 'GREEN' if (golden_tests or golden_assets) else 'RED',
    'bench': 'GREEN' if benches else 'RED',
    'metrics': 'GREEN' if metrics_hits else 'RED',
    'ci': 'GREEN' if ci_workflows else 'RED',
}
kill = sum(1 for v in exits.values() if v != 'GREEN')

summary = {
    'repo_root': str(ROOT),
    'exits': exits,
    'kill_criteria_count': kill,
    'overall': 'GREEN' if kill == 0 else 'RED',
    'unit_tests': sorted(unit),
    'property_signals': sorted(set(property_tests)),
    'golden_tests': sorted(golden_tests),
    'golden_assets': sorted(golden_assets),
    'benches': sorted(benches),
    'metrics_signals': sorted(set(metrics_hits)),
    'ci_workflows': sorted(ci_workflows),
}

(OUT / 'orr_static_summary.json').write_text(
    json.dumps(summary, indent=2, ensure_ascii=False),
    encoding='utf-8'
)
print(json.dumps({'overall': summary['overall'], 'kill': kill}, indent=2))
