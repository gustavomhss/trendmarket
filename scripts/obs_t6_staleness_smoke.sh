#!/usr/bin/env bash
set -Eeuo pipefail
mkdir -p out/obs_gatecheck/logs out/obs_gatecheck/evidence
if ! curl -fsS http://127.0.0.1:13133/healthz > /dev/null; then
  printf 'COLLECTOR_HEALTH_FAIL\n'
  exit 2
fi
if ! curl -fsS http://127.0.0.1:9090/-/ready > /dev/null; then
  printf 'PROM_READY_FAIL\n'
  exit 3
fi
emit_status=0
ce_obs_demo --emit oracle 5 --emit cdc_topic:orders 10 > out/obs_gatecheck/logs/obs6_staleness.txt 2>&1 || emit_status=$?
if [ "$emit_status" -ne 0 ]; then
  printf 'STALE_SMOKE_FAIL:emit\n'
  exit 1
fi
raw_file=out/obs_gatecheck/evidence/t5_freshness_raw.json
rec_file=out/obs_gatecheck/evidence/t5_freshness_recording.json
curl -fsS "http://127.0.0.1:9090/api/v1/query?query=max%20by%20(source)%20(data_freshness_seconds)" > "$raw_file"
curl -fsS "http://127.0.0.1:9090/api/v1/query?query=ce:data_freshness_seconds:max_by_source" > "$rec_file"
raw_count=$(grep -c '"metric"' "$raw_file")
if [ "$raw_count" -lt 2 ]; then
  printf 'STALE_SMOKE_FAIL:raw\n'
  exit 4
fi
rec_count=$(grep -c '"metric"' "$rec_file")
if [ "$rec_count" -lt 2 ]; then
  printf 'STALE_SMOKE_FAIL:rec\n'
  exit 5
fi
summary_file=out/obs_gatecheck/evidence/t5_smoke_summary.json
printf '{"obs":"OBS-6","thread":"T5","emitted":["oracle:5s","cdc_topic:orders:10s"],"raw_series_count":%s,"recording_series_count":%s,"ok":true}\n' "$raw_count" "$rec_count" > "$summary_file"
printf 'STALE_SMOKE_OK\n'
