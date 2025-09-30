#!/usr/bin/env bash
set -Eeuo pipefail
OUT="out/orr_gatecheck/evidence/ci"
LOG="out/orr_gatecheck/logs"
mkdir -p "$OUT" "$LOG"
if ! command -v gh >/dev/null 2>&1; then
  echo '{"error":"gh CLI ausente","status":"UNKNOWN"}' > "$OUT/run_summary.json"
  exit 0
fi
# Descobrir último run desta branch
BR=$(git rev-parse --abbrev-ref HEAD)
set +e
RUN_JSON=$(gh run list --branch "$BR" --workflow CI --limit 1 --json databaseId,headBranch,conclusion,status,durationMS,updatedAt) || true
set -e
[ -n "$RUN_JSON" ] || RUN_JSON='[]'
echo "$RUN_JSON" > "$OUT/run_summary.json"
