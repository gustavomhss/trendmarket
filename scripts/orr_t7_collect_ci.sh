#!/usr/bin/env bash
set -Eeuo pipefail
OUT="out/orr_gatecheck/evidence/ci"
mkdir -p "$OUT"
TMP="$(mktemp)"
if ! gh auth status >/dev/null 2>&1; then
  echo "ERROR: gh not authenticated. Run: gh auth login -s repo,workflow -h github.com" >&2
  exit 2
fi
gh run list --limit 20 \
  --json status,conclusion,workflowName,displayTitle,headBranch,headSha,createdAt,startedAt,updatedAt,url,number \
  > "$TMP"
jq '[ .[] | select(.headBranch=="main") | select(.status=="completed") | . + {duration_seconds: (( (.updatedAt|fromdateiso8601) - (.startedAt|fromdateiso8601) ) // 0)} ] | .[0:1]' "$TMP" > "$OUT/run_summary.json"
jq -n --argjson count "$(jq 'length' "$OUT/run_summary.json")" '{count:$count}'
