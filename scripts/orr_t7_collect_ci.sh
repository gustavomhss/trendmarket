#!/usr/bin/env bash
set -Eeuo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck/evidence/ci"
if ! mkdir -p "$OUT" 2>/dev/null; then
  echo '{"step":"T7","error":"read_only"}'
  exit 95
fi

if ! TMP_WRITE_TEST="$(mktemp "$OUT/.write_test.XXXXXX" 2>/dev/null)"; then
  echo '{"step":"T7","error":"read_only"}'
  exit 95
fi
rm -f "$TMP_WRITE_TEST"

if ! command -v gh >/dev/null 2>&1; then
  echo '{"step":"T7","error":"gh_unavailable"}'
  exit 2
fi

if ! gh auth status -h github.com -t >/dev/null 2>&1; then
  echo '{"step":"T7","error":"gh_unauthenticated"}'
  exit 2
fi

TMP_JSON="$(mktemp)"
cleanup() {
  rm -f "$TMP_JSON"
}
trap cleanup EXIT INT TERM

cd "$ROOT"
gh run list --limit 20 \
  --json status,conclusion,workflowName,displayTitle,headBranch,headSha,createdAt,startedAt,updatedAt,url,number \
  >"$TMP_JSON"

TMP_OUT="$(mktemp "$OUT/run_summary.json.XXXXXX")"
jq '[ .[]
      | select(.headBranch=="main")
      | select(.status=="completed")
      | . + {duration_seconds: ((try ((.updatedAt|fromdateiso8601) - (.startedAt|fromdateiso8601)) catch 0) | floor)}
    ]
    | sort_by(.updatedAt)
    | reverse
    | .[:1]
  ' "$TMP_JSON" >"$TMP_OUT"
mv "$TMP_OUT" "$OUT/run_summary.json"
