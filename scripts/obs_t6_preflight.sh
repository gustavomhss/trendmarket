#!/usr/bin/env bash
set -Eeuo pipefail

base_dir="out/obs_gatecheck"
log_dir="$base_dir/logs"
evidence_dir="$base_dir/evidence"
mkdir -p "$log_dir" "$evidence_dir"

collector_url="http://127.0.0.1:13133/healthz"
prometheus_url="http://127.0.0.1:9090/api/v1/status/runtimeinfo"
log_file="$log_dir/t0_preflight.txt"
manifest_file="$evidence_dir/t0_preflight.json"

uname_value=$(uname -a)
ts_start=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

collector_http_code="0"
if collector_response=$(curl -s -o /dev/null -w "%{http_code}" "$collector_url"); then
  collector_http_code="$collector_response"
fi

prometheus_http_code="0"
if prometheus_response=$(curl -s -o /dev/null -w "%{http_code}" "$prometheus_url"); then
  prometheus_http_code="$prometheus_response"
fi

collector_ok="false"
if [ "$collector_http_code" = "200" ]; then
  collector_ok="true"
fi

prometheus_ok="false"
if [ "$prometheus_http_code" = "200" ]; then
  prometheus_ok="true"
fi

collector_token="COLLECTOR_HEALTH_FAIL"
if [ "$collector_ok" = "true" ]; then
  collector_token="COLLECTOR_HEALTH_OK"
fi

prometheus_token="PROM_READY_FAIL"
if [ "$prometheus_ok" = "true" ]; then
  prometheus_token="PROM_READY_OK"
fi

printf "%s\n" "$collector_token"
printf "%s\n" "$prometheus_token"

ts_end=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

printf "ts_start: %s\n" "$ts_start" > "$log_file"
printf "collector_url: %s\n" "$collector_url" >> "$log_file"
printf "collector_http_code: %s\n" "$collector_http_code" >> "$log_file"
printf "collector_token: %s\n" "$collector_token" >> "$log_file"
printf "prometheus_url: %s\n" "$prometheus_url" >> "$log_file"
printf "prometheus_http_code: %s\n" "$prometheus_http_code" >> "$log_file"
printf "prometheus_token: %s\n" "$prometheus_token" >> "$log_file"
printf "ts_end: %s\n" "$ts_end" >> "$log_file"

printf '{\n' > "$manifest_file"
printf '  "obs": "OBS-6",\n' >> "$manifest_file"
printf '  "thread": "T0",\n' >> "$manifest_file"
printf '  "script": "scripts/obs_t6_preflight.sh",\n' >> "$manifest_file"
printf '  "ts_start": "%s",\n' "$ts_start" >> "$manifest_file"
printf '  "ts_end": "%s",\n' "$ts_end" >> "$manifest_file"
printf '  "collector": {\n' >> "$manifest_file"
printf '    "url": "%s",\n' "$collector_url" >> "$manifest_file"
printf '    "http_code": %s,\n' "$collector_http_code" >> "$manifest_file"
printf '    "ok": %s\n' "$collector_ok" >> "$manifest_file"
printf '  },\n' >> "$manifest_file"
printf '  "prometheus": {\n' >> "$manifest_file"
printf '    "url": "%s",\n' "$prometheus_url" >> "$manifest_file"
printf '    "http_code": %s,\n' "$prometheus_http_code" >> "$manifest_file"
printf '    "ok": %s\n' "$prometheus_ok" >> "$manifest_file"
printf '  },\n' >> "$manifest_file"
printf '  "host": {\n' >> "$manifest_file"
printf '    "uname": "%s"\n' "$uname_value" >> "$manifest_file"
printf '  },\n' >> "$manifest_file"
printf '  "tokens": [\n' >> "$manifest_file"
printf '    "%s",\n' "$collector_token" >> "$manifest_file"
printf '    "%s"\n' "$prometheus_token" >> "$manifest_file"
printf '  ]\n' >> "$manifest_file"
printf '}\n' >> "$manifest_file"

exit_code=0
if [ "$collector_ok" != "true" ]; then
  exit_code=2
elif [ "$prometheus_ok" != "true" ]; then
  exit_code=3
fi

exit "$exit_code"
