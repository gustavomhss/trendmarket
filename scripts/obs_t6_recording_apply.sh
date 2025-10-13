#!/usr/bin/env bash
set -Eeuo pipefail
rule_file="ops/prometheus/rules/core.rules.yml"
group_name="ce-core-data"
record="ce:data_freshness_seconds:max_by_source"
expr="max by (source) (data_freshness_seconds)"
out_dir="out/obs_gatecheck"
log_dir="$out_dir/logs"
evidence_dir="$out_dir/evidence"
log_file="$log_dir/t4_recording.txt"
rules_json="$evidence_dir/t4_rules_dump.json"
query_json="$evidence_dir/t4_recording_query.json"
summary_json="$evidence_dir/t4_recording_rules.json"
mkdir -p "$log_dir" "$evidence_dir"
: > "$log_file"
log() {
  printf '%s\n' "$1" >> "$log_file"
  printf '%s\n' "$1"
}
if [ ! -f "$rule_file" ]; then
  log "FILE_CHECK:missing"
  log "RECORDING_FAIL:file"
  exit 8
fi
if ! grep -qF "record: $record" "$rule_file"; then
  if grep -qF "name: $group_name" "$rule_file"; then
    tmp_file="$rule_file.tmp"
    awk -v record="$record" -v expr="$expr" '
      function emit_rule() {
        print "      - record: " record
        print "        expr: " expr
        inserted=1
      }
      /^  - name: ce-core-data$/ {
        print
        in_group=1
        in_rules=0
        inserted=0
        next
      }
      in_group && /^    rules:/ {
        print
        in_rules=1
        next
      }
      in_group && in_rules && /^      - / {
        print
        if($0 ~ record) inserted=1
        next
      }
      {
        if(in_group && in_rules && inserted==0) {
          emit_rule()
          in_group=0
          in_rules=0
        } else if(in_group && in_rules==0) {
          print "    rules:"
          emit_rule()
          in_group=0
        }
        print
      }
      END {
        if(in_group) {
          if(in_rules && inserted==0) {
            emit_rule()
          } else if(in_rules==0) {
            print "    rules:"
            emit_rule()
          }
        }
      }
    ' "$rule_file" > "$tmp_file"
    mv "$tmp_file" "$rule_file"
  else
    printf '\n  - name: %s\n    rules:\n      - record: %s\n        expr: %s\n' "$group_name" "$record" "$expr" >> "$rule_file"
  fi
fi
if ! grep -qF "record: $record" "$rule_file"; then
  log "FILE_CHECK:write_failed"
  log "RECORDING_FAIL:file"
  exit 8
fi
log "FILE_CHECK:ok"
reload_code=$(curl -s -o /dev/null -w '%{http_code}' -X POST http://127.0.0.1:9090/-/reload || printf '000')
log "RELOAD_HTTP:$reload_code"
rules_tmp="$rules_json.tmp"
rules_code=$(curl -s -o "$rules_tmp" -w '%{http_code}' http://127.0.0.1:9090/api/v1/rules || printf '000')
if [ -f "$rules_tmp" ]; then
  mv "$rules_tmp" "$rules_json"
else
  printf '{}' > "$rules_json"
fi
log "RULES_HTTP:$rules_code"
query_tmp="$query_json.tmp"
query_code=$(curl -s -o "$query_tmp" -w '%{http_code}' --get --data-urlencode "query=$expr" http://127.0.0.1:9090/api/v1/query || printf '000')
if [ -f "$query_tmp" ]; then
  mv "$query_tmp" "$query_json"
else
  printf '{}' > "$query_json"
fi
log "QUERY_HTTP:$query_code"
rule_present_api=0
if [ "$rules_code" = "200" ] && grep -qF "$record" "$rules_json"; then
  rule_present_api=1
fi
result_count=0
if [ "$query_code" = "200" ]; then
  result_count=$(grep -c '"metric"' "$query_json" || printf '0')
fi
present_value=false
if [ "$rule_present_api" -eq 1 ]; then
  present_value=true
fi
ok_value=false
if [ "$rule_present_api" -eq 1 ] && [ "$result_count" -ge 1 ]; then
  ok_value=true
fi
printf '{\n' > "$summary_json"
printf '  "file": "%s",\n' "$rule_file" >> "$summary_json"
printf '  "group": "%s",\n' "$group_name" >> "$summary_json"
printf '  "record": "%s",\n' "$record" >> "$summary_json"
printf '  "expr": "%s",\n' "$expr" >> "$summary_json"
printf '  "present": %s,\n' "$present_value" >> "$summary_json"
printf '  "query_result_count": %s,\n' "$result_count" >> "$summary_json"
printf '  "ok": %s\n' "$ok_value" >> "$summary_json"
printf '}\n' >> "$summary_json"
if [ "$rule_present_api" -eq 1 ] && [ "$result_count" -ge 1 ]; then
  log "RECORDING_OK"
  exit 0
fi
if [ "$rule_present_api" -eq 0 ]; then
  log "RECORDING_FAIL:missing"
  exit 7
fi
log "RECORDING_FAIL:query"
exit 3
