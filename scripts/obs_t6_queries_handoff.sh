#!/usr/bin/env bash
set -Eeuo pipefail
CATALOG="ops/grafana/queries_obs6.txt"
OUT_DIR="out/obs_gatecheck"
LOG_DIR="$OUT_DIR/logs"
EVIDENCE_DIR="$OUT_DIR/evidence"
PROM_URL="${PROM_URL:-http://localhost:9090}"
trim() {
  local s="$1"
  while [ "${s# }" != "$s" ]; do s=${s# }; done
  while [ "${s% }" != "$s" ]; do s=${s% }; done
  printf '%s' "$s"
}
mkdir -p "$LOG_DIR" "$EVIDENCE_DIR"
items=0
nonempty=0
http_fail=0
while IFS= read -r line || [ -n "$line" ]; do
  if [ -z "$line" ]; then
    continue
  fi
  items=$((items + 1))
  expr=${line%%|*}
  expr=$(trim "$expr")
  evidence_file="$EVIDENCE_DIR/t6_query_${items}.json"
  log_file="$LOG_DIR/t6_query_${items}.log"
  if curl --fail --silent --show-error --get "$PROM_URL/api/v1/query" --data-urlencode "query=$expr" > "$evidence_file" 2> "$log_file"; then
    if ! grep -q '"result":\[\]' "$evidence_file"; then
      nonempty=$((nonempty + 1))
    fi
  else
    http_fail=1
    : > "$evidence_file"
  fi
done < "$CATALOG"
ok="false"
if [ "$nonempty" -ge 2 ]; then
  ok="true"
fi
summary_file="$EVIDENCE_DIR/t6_queries_summary.json"
printf '{\n  "obs": "OBS-6",\n  "thread": "T6",\n  "items": %d,\n  "nonempty": %d,\n  "ok": %s\n}\n' "$items" "$nonempty" "$ok" > "$summary_file"
if [ "$http_fail" -ne 0 ]; then
  printf 'CATALOG_FAIL:http\n'
  exit 9
fi
if [ "$ok" = "true" ]; then
  printf 'CATALOG_OK\n'
  exit 0
fi
printf 'CATALOG_FAIL:empty\n'
exit 9
