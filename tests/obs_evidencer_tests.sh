#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

scripts/obs_evidencer.sh --ops 5 --prom

MANIFEST="out/obs_gatecheck/evidence/obs1_sdk.json"
SMOKE_LOG="out/obs_gatecheck/logs/obs1_smoke.txt"
METRICS_SAMPLE="out/obs_gatecheck/logs/obs1_metrics_sample.txt"

[ -s "$MANIFEST" ] || { echo "missing manifest" >&2; exit 1; }
[ -s "$SMOKE_LOG" ] || { echo "missing smoke log" >&2; exit 1; }
[ -s "$METRICS_SAMPLE" ] || { echo "missing metrics sample" >&2; exit 1; }

python3 - <<'PY'
import json
from pathlib import Path

manifest_path = Path("out/obs_gatecheck/evidence/obs1_sdk.json")
smoke_path = Path("out/obs_gatecheck/logs/obs1_smoke.txt")

data = json.loads(manifest_path.read_text())
if data["metrics"]["amm_op_latency_seconds"]["buckets_nonzero"] < 3:
    raise SystemExit("expected >=3 non-zero buckets")
if data["logs"]["lines_json"] < 1:
    raise SystemExit("expected >=1 structured log line")
if not data["logs"]["sample"]:
    raise SystemExit("log sample missing")

valid_ops = {"swap", "pricing", "cdc_consume"}
found = False
with smoke_path.open('r', encoding='utf-8', errors='replace') as fh:
    for line in fh:
        line = line.strip()
        if not line:
            continue
        try:
            payload = json.loads(line)
        except json.JSONDecodeError:
            continue
        if 'trace_id' in payload and payload.get('op') in valid_ops:
            found = True
            break

if not found:
    raise SystemExit("no log line with expected op + trace_id")
PY

echo "obs_evidencer smoke test passed"
