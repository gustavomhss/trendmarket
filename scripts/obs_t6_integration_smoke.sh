#!/usr/bin/env bash
set -Eeuo pipefail
if command -v ce_obs_demo >/dev/null 2>&1; then
    ce_obs_demo --emit oracle 5 --emit cdc_topic:orders 10
else
    cargo run --bin ce_obs_demo -- --emit oracle 5 --emit cdc_topic:orders 10
fi
mkdir -p out/obs_gatecheck/evidence
response=$(curl -s --get 'http://127.0.0.1:9090/api/v1/query' --data-urlencode 'query=max by (source) (data_freshness_seconds)')
printf '%s' "$response" > out/obs_gatecheck/evidence/t2_integration_checks.json
count=$(python3 - <<'PY'
import json
import sys
try:
    payload = json.load(sys.stdin)
except json.JSONDecodeError:
    sys.exit(1)
if payload.get("status") != "success":
    sys.exit(1)
result = payload.get("data", {}).get("result", [])
print(len(result))
PY
<<<"$response")
if [ "$count" -ge 2 ]; then
    echo INTEGRATION_OK
else
    echo INTEGRATION_FAIL:series
    exit 2
fi
