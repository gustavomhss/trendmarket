#!/usr/bin/env bash
set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck"
LOG_DIR="$OUT/logs"
EVI_DIR="$OUT/evidence/property"
STEP="T3"

fail_read_only() {
  printf '{ "step":"%s", "error":"read_only" }\n' "$STEP"
  exit 95
}

require_write_access() {
  local target="$1"
  local dir="$target"
  if [ ! -d "$dir" ]; then
    dir="$(dirname "$dir")"
  fi
  while [ ! -d "$dir" ] && [ "$dir" != "/" ]; do
    dir="$(dirname "$dir")"
  done
  if [ ! -w "$dir" ]; then
    fail_read_only
  fi
  local probe
  if ! probe="$(mktemp "$dir/.writecheck.XXXXXX" 2>/dev/null)"; then
    fail_read_only
  fi
  rm -f "$probe"
}

require_write_access "$OUT"
require_write_access "$LOG_DIR"
require_write_access "$EVI_DIR"

mkdir -p "$LOG_DIR" "$EVI_DIR"

TMPDIS=""
LOG_TMP=""
SEEDS_TMP=""

restore_bin() {
  if [ -n "$TMPDIS" ] && [ -f "$TMPDIS" ]; then
    mv "$TMPDIS" "$ROOT/src/bin/telemetry_smoke.rs"
    TMPDIS=""
  fi
}

cleanup() {
  restore_bin
  if [ -n "$LOG_TMP" ] && [ -f "$LOG_TMP" ]; then
    rm -f "$LOG_TMP"
    LOG_TMP=""
  fi
  if [ -n "$SEEDS_TMP" ] && [ -f "$SEEDS_TMP" ]; then
    rm -f "$SEEDS_TMP"
    SEEDS_TMP=""
  fi
}

trap 'cleanup' EXIT INT TERM

if [ -f "$ROOT/src/bin/telemetry_smoke.rs" ]; then
  TMPDIS="$ROOT/src/bin/telemetry_smoke.rs.bak.$$"
  mv "$ROOT/src/bin/telemetry_smoke.rs" "$TMPDIS"
fi

cd "$ROOT"
LOG_FILE="$LOG_DIR/cargo_test_property.txt"
LOG_TMP="$(mktemp "$LOG_FILE.XXXXXX")"
set +e
cargo test --test property -- --nocapture | tee "$LOG_TMP"
RC=${PIPESTATUS[0]}
set -e

python3 - "$LOG_TMP" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY

mv "$LOG_TMP" "$LOG_FILE"
LOG_TMP=""

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
python3 - "$SEEDS_TMP" <<'PY'
import os
import sys

path = sys.argv[1]
fd = os.open(path, os.O_RDONLY)
try:
    os.fsync(fd)
finally:
    os.close(fd)
PY
mv "$SEEDS_TMP" "$EVI_DIR/seeds.jsonl"
SEEDS_TMP=""

cleanup
trap - EXIT INT TERM || true
exit $RC
