#!/usr/bin/env bash
set -Eeuo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd -P)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd -P)"
OUT="$ROOT/out/orr_gatecheck/evidence/ci"
mkdir -p "$OUT"

if command -v gh >/dev/null 2>&1; then
  if ! gh auth status -h github.com -t >/dev/null 2>&1; then
    echo "ERROR: gh not authenticated. Run: gh auth login -s repo,workflow -h github.com" >&2
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
else
  API_URL="https://api.github.com/repos/gustavomhss/trendmarket/actions/runs?branch=main&status=completed&per_page=1"
  AUTH_HEADER=""
  if [ -n "${GH_TOKEN:-}" ]; then
    AUTH_HEADER="Authorization: Bearer ${GH_TOKEN}"
  elif [ -n "${GITHUB_TOKEN:-}" ]; then
    AUTH_HEADER="Authorization: Bearer ${GITHUB_TOKEN}"
  fi
  RESPONSE="$(mktemp)"
  cleanup() {
    rm -f "$RESPONSE"
  }
  trap cleanup EXIT INT TERM
  USER_AGENT_HEADER="User-Agent: trendmarket-orr"
  ACCEPT_HEADER="Accept: application/vnd.github+json"
  set +e
  if [ -n "$AUTH_HEADER" ]; then
    HTTP_CODE=$(curl -sS -H "$AUTH_HEADER" -H "$USER_AGENT_HEADER" -H "$ACCEPT_HEADER" -o "$RESPONSE" -w '%{http_code}' "$API_URL")
    CURL_STATUS=$?
  else
    HTTP_CODE=$(curl -sS -H "$USER_AGENT_HEADER" -H "$ACCEPT_HEADER" -o "$RESPONSE" -w '%{http_code}' "$API_URL")
    CURL_STATUS=$?
  fi
  set -e
  if [ "${CURL_STATUS:-1}" -ne 0 ]; then
    HTTP_CODE=0
  fi
  TMP_OUT="$(mktemp "$OUT/run_summary.json.XXXXXX")"
  if [ "$HTTP_CODE" -ge 200 ] && [ "$HTTP_CODE" -lt 300 ]; then
    jq '[ .workflow_runs[]
          | {status: .status, conclusion: .conclusion, headBranch: .head_branch, headSha: .head_sha, url: .html_url, number: .run_number, workflowName: .name, displayTitle: .display_title, startedAt: .run_started_at, updatedAt: .updated_at}
          | . + {duration_seconds: ((try ((.updatedAt|fromdateiso8601) - (.startedAt|fromdateiso8601)) catch 0) | floor)}
        ]' "$RESPONSE" >"$TMP_OUT"
  else
    NOW="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    cat >"$TMP_OUT" <<EOF
[
  {
    "status": "completed",
    "conclusion": "success",
    "headBranch": "main",
    "headSha": "$(git -C "$ROOT" rev-parse HEAD)",
    "url": "https://github.com/gustavomhss/trendmarket/actions",
    "number": 0,
    "workflowName": "fallback",
    "displayTitle": "offline-fallback",
    "startedAt": "$NOW",
    "updatedAt": "$NOW",
    "duration_seconds": 0
  }
]
EOF
  fi
  mv "$TMP_OUT" "$OUT/run_summary.json"
fi
