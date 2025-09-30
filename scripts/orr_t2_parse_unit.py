#!/usr/bin/env python3
import json
import pathlib
import re

LOG = pathlib.Path('out/orr_gatecheck/logs/cargo_test_unit.txt')
text = LOG.read_text(encoding='utf-8', errors='ignore')
rx = re.compile(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out")
passed = failed = ignored = measured = filtered = 0
status = "GREEN"
match_found = False
for match in rx.finditer(text):
    match_found = True
    if match.group(1) != "ok":
        status = "RED"
    passed += int(match.group(2))
    failed += int(match.group(3))
    ignored += int(match.group(4))
    measured += int(match.group(5))
    filtered += int(match.group(6))

if not match_found:
    status = "UNKNOWN"

outp = {
    "status": status,
    "passed": passed,
    "failed": failed,
    "ignored": ignored,
    "measured": measured,
    "filtered_out": filtered,
}
PATH = pathlib.Path('out/orr_gatecheck/evidence/unit/summary.json')
PATH.write_text(json.dumps(outp, indent=2), encoding='utf-8')
print(json.dumps(outp, indent=2))
