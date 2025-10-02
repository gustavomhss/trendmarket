#!/usr/bin/env python3
import json
import os
import pathlib
import re
import sys
import tempfile


def _nearest_existing_dir(path: pathlib.Path) -> pathlib.Path | None:
    current = path
    while True:
        if current.exists():
            if current.is_dir():
                return current
            parent = current.parent
            return parent if parent != current else None
        parent = current.parent
        if parent == current:
            return None
        current = parent


def _is_dir_writable(path: pathlib.Path) -> bool:
    existing_dir = _nearest_existing_dir(path)
    if existing_dir is None:
        return False
    return os.access(existing_dir, os.W_OK | os.X_OK)

SCRIPT_DIR = pathlib.Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
LOG = ROOT / 'out' / 'orr_gatecheck' / 'logs' / 'cargo_test_unit.txt'

if not LOG.exists():
    sys.stderr.write('ERROR: missing unit test log at %s\n' % LOG)
    sys.exit(1)

text = LOG.read_text(encoding='utf-8', errors='ignore')
rx = re.compile(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")
passed = failed = ignored = measured = filtered = 0
match_found = False

for match in rx.finditer(text):
    match_found = True
    passed += int(match.group(2))
    failed += int(match.group(3))
    ignored += int(match.group(4))
    measured += int(match.group(5))
    filtered += int(match.group(6))

status = 'GREEN' if match_found and failed == 0 else 'RED'

outp = {
    'status': status,
    'passed': passed,
    'failed': failed,
    'ignored': ignored,
    'measured': measured,
    'filtered_out': filtered,
}

evidence_dir = ROOT / 'out' / 'orr_gatecheck' / 'evidence' / 'unit'
if not _is_dir_writable(evidence_dir):
    print(json.dumps(outp, separators=(',', ':')))
    sys.exit(95)
evidence_dir.mkdir(parents=True, exist_ok=True)
target = evidence_dir / 'summary.json'

with tempfile.NamedTemporaryFile('w', encoding='utf-8', delete=False, dir=str(evidence_dir), prefix='summary.', suffix='.json') as tmp:
    json.dump(outp, tmp, indent=2)
    tmp.flush()
    os.fsync(tmp.fileno())
temp_path = pathlib.Path(tmp.name)
temp_path.replace(target)

print(json.dumps(outp, indent=2))
