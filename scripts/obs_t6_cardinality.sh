#!/usr/bin/env bash
set -Eeuo pipefail
mkdir -p out/obs_gatecheck/logs out/obs_gatecheck/evidence
now=$(date +%s)
start=$((now-3600))
end=$now
base_url="http://localhost:9090"
if ! curl -sS "${base_url}/api/v1/series?match[]=data_freshness_seconds&start=${start}&end=${end}" -o out/obs_gatecheck/evidence/t7_series_freshness.json; then
  printf '' > out/obs_gatecheck/evidence/t7_series_freshness.json
fi
if ! curl -sS "${base_url}/api/v1/series?match[]=ce%3Adata_freshness_seconds%3Amax_by_source&start=${start}&end=${end}" -o out/obs_gatecheck/evidence/t7_series_recording.json; then
  printf '' > out/obs_gatecheck/evidence/t7_series_recording.json
fi
if python3 scripts/obs_cardinality_check.py; then
  :
else
  status=$?
  if [ "$status" -ne 5 ]; then
    exit "$status"
  fi
fi
ok_value=$(grep -m1 '"ok"' out/obs_gatecheck/evidence/t7_cardinality.json | cut -d: -f2 | tr -d ' ,')
reason=$(grep -m1 '"reason"' out/obs_gatecheck/evidence/t7_cardinality.json | cut -d: -f2- | tr -d ' "')
if [ "$ok_value" = "true" ]; then
  printf "CARDINALITY_OK\n"
  exit 0
fi
if [ -z "$reason" ]; then
  reason="http"
fi
printf "CARDINALITY_FAIL:%s\n" "$reason"
exit 5
