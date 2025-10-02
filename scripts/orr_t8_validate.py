#!/usr/bin/env python3
import json
import os
import pathlib
import sys
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


def rd_safe(path: pathlib.Path):
    return load_json(path)


unit_summary = rd_safe(EVIDENCE_DIR / 'unit' / 'summary.json')
property_summary = rd_safe(EVIDENCE_DIR / 'property' / 'summary.json')
goldens_summary = rd_safe(EVIDENCE_DIR / 'goldens' / 'summary.json')
bench_summary = rd_safe(EVIDENCE_DIR / 'bench' / 'baseline' / 'criterion_summary.json')
metrics_ports = rd_safe(EVIDENCE_DIR / 'metrics' / 'ports.json')
smoke_exists = (EVIDENCE_DIR / 'metrics' / 'smoke.txt').exists()
ci_runs = rd_safe(EVIDENCE_DIR / 'ci' / 'run_summary.json')


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
summary_json = json.dumps(summary, indent=2)

try:
    tmp_file = tempfile.NamedTemporaryFile(
        'w',
        encoding='utf-8',
        delete=False,
        dir=str(EVIDENCE_DIR),
        prefix='orr_final_summary.',
        suffix='.json',
    )
except OSError:
    print(summary_json)
    sys.exit(95)

with tmp_file as tmp:
    tmp.write(summary_json)
    tmp.flush()
    os.fsync(tmp.fileno())

TEMP_PATH = pathlib.Path(tmp_file.name)
TEMP_PATH.replace(TARGET)

print(json.dumps({'overall': overall, 'kill': kill_count}))
