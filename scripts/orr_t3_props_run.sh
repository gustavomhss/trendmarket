#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
LOG_DIR="$OUT/logs"
EVI_DIR="$OUT/evidence/property"
mkdir -p "$LOG_DIR" "$EVI_DIR"

TMPDIS=""
restore_bin() {
  if [ -n "$TMPDIS" ] && [ -f "$TMPDIS" ]; then
    mv "$TMPDIS" "$ROOT/src/bin/telemetry_smoke.rs"
    TMPDIS=""
  fi
}

if [ -f "$ROOT/src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="$ROOT/src/bin/telemetry_smoke.rs.bak.$$"
  mv "$ROOT/src/bin/telemetry_smoke.rs" "$TMPDIS"
  trap 'restore_bin' EXIT INT TERM
fi

cd "$ROOT"
LOG_FILE="$LOG_DIR/cargo_test_property.txt"
cargo test --test property -- --nocapture | tee "$LOG_FILE"
RC=${PIPESTATUS[0]}
restore_bin
trap - EXIT INT TERM || true

python3 - "$ROOT" "$RC" <<'PY'
import json
import os
import pathlib
import re
import sys
import tempfile

root = pathlib.Path(sys.argv[1])
out_dir = root / 'out' / 'orr_gatecheck'
log_path = out_dir / 'logs' / 'cargo_test_property.txt'
exit_code = int(sys.argv[2])
text = log_path.read_text(encoding='utf-8', errors='ignore')
pattern = re.compile(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;")
passed = failed = 0
for match in pattern.finditer(text):
    passed += int(match.group(2))
    failed += int(match.group(3))
status = 'GREEN' if exit_code == 0 and failed == 0 else 'RED'
summary = {
    'status': status,
    'passed': passed,
    'failed': failed,
}
evidence_dir = out_dir / 'evidence' / 'property'
evidence_dir.mkdir(parents=True, exist_ok=True)
summary_target = evidence_dir / 'summary.json'
with tempfile.NamedTemporaryFile('w', encoding='utf-8', delete=False, dir=str(evidence_dir), prefix='summary.', suffix='.json') as tmp:
    json.dump(summary, tmp, indent=2)
    tmp.flush()
    os.fsync(tmp.fileno())
summary_tmp = pathlib.Path(tmp.name)
summary_tmp.replace(summary_target)
PY

SEEDS_TMP="$(mktemp "$EVI_DIR/seeds.jsonl.XXXXXX")"
if grep -E '^seed:[0-9]+' "$LOG_FILE" >"$SEEDS_TMP"; then
  :
else
  : >"$SEEDS_TMP"
fi
mv "$SEEDS_TMP" "$EVI_DIR/seeds.jsonl"

exit $RC
