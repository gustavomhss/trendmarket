#!/usr/bin/env python3
import json
import os
import pathlib
import tempfile
from datetime import datetime, timezone

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
OUT = ROOT / 'out' / 'orr_gatecheck'
EVIDENCE_DIR = OUT / 'evidence'
EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)


def load_json(path: pathlib.Path):
    if not path.exists():
        return None
    try:
        return json.loads(path.read_text(encoding='utf-8'))
    except Exception:
        return None


unit_summary = load_json(EVIDENCE_DIR / 'unit' / 'summary.json')
property_summary = load_json(EVIDENCE_DIR / 'property' / 'summary.json')
goldens_summary = load_json(EVIDENCE_DIR / 'goldens' / 'summary.json')
bench_summary = load_json(EVIDENCE_DIR / 'bench' / 'baseline' / 'criterion_summary.json')
metrics_ports = load_json(EVIDENCE_DIR / 'metrics' / 'ports.json')
smoke_exists = (EVIDENCE_DIR / 'metrics' / 'smoke.txt').exists()
ci_runs = load_json(EVIDENCE_DIR / 'ci' / 'run_summary.json')


def unit_green(data):
    return bool(data) and data.get('failed', 1) == 0


def property_green(data):
    return bool(data) and data.get('failed', 1) == 0


def goldens_green(data):
    return bool(data) and data.get('status') == 'GREEN' and data.get('mismatch') == 0


def bench_green(data):
    return bool(data) and data.get('count', 0) > 0


def metrics_green(has_smoke, ports):
    return bool(has_smoke) and isinstance(ports, dict)


def ci_green(runs):
    return isinstance(runs, list) and len(runs) > 0


statuses = {
    'unit': 'GREEN' if unit_green(unit_summary) else 'RED',
    'property': 'GREEN' if property_green(property_summary) else 'RED',
    'goldens': 'GREEN' if goldens_green(goldens_summary) else 'RED',
    'bench': 'GREEN' if bench_green(bench_summary) else 'RED',
    'metrics': 'GREEN' if metrics_green(smoke_exists, metrics_ports) else 'RED',
    'ci': 'GREEN' if ci_green(ci_runs) else 'RED',
}

kill_count = sum(1 for value in statuses.values() if value != 'GREEN')
overall = 'GREEN' if kill_count == 0 else 'RED'

summary = {
    'timestamp': datetime.now(timezone.utc).isoformat(),
    'overall': overall,
    'kill_criteria_count': kill_count,
    'exits': statuses,
}

TARGET = EVIDENCE_DIR / 'orr_final_summary.json'
with tempfile.NamedTemporaryFile('w', encoding='utf-8', delete=False, dir=str(EVIDENCE_DIR), prefix='orr_final_summary.', suffix='.json') as tmp:
    json.dump(summary, tmp, indent=2)
    tmp.flush()
    os.fsync(tmp.fileno())
TEMP_PATH = pathlib.Path(tmp.name)
TEMP_PATH.replace(TARGET)

print(json.dumps({'overall': overall, 'kill': kill_count}))
