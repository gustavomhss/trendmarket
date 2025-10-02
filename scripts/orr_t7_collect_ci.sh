#!/usr/bin/env bash
set -Eeuo pipefail
OUT="out/orr_gatecheck/evidence/ci"
mkdir -p "$OUT"
TMP="$(mktemp)"
# Exige gh autenticado; se não, falha com mensagem clara
if ! gh auth status >/dev/null 2>&1; then
  echo "ERROR: gh not authenticated. Run: gh auth login -s repo,workflow -h github.com" >&2
  exit 2
fi
# Descobre o branch atual da forma mais resiliente possível
BRANCH=""
if command -v git >/dev/null 2>&1; then
  BRANCH="$(git branch --show-current 2>/dev/null || true)"
  if [ -z "$BRANCH" ]; then
    BRANCH="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)"
  fi
fi
if [ -z "$BRANCH" ] || [ "$BRANCH" = "HEAD" ]; then
  if [ -n "${GITHUB_HEAD_REF:-}" ]; then
    BRANCH="$GITHUB_HEAD_REF"
  elif [ -n "${BRANCH_NAME:-}" ]; then
    BRANCH="$BRANCH_NAME"
  else
    BRANCH="main"
  fi
fi
# Coleta runs mais recentes (branch atual, completados)
gh run list --limit 20 --branch "$BRANCH" \
  --json status,conclusion,workflowName,displayTitle,headBranch,headSha,createdAt,startedAt,updatedAt,url,number \
  > "$TMP"
# Filtra e calcula duração
jq --arg branch "$BRANCH" '[ .[]
      | select(.headBranch==$branch)
      | select(.status=="completed")
      | . + {duration_seconds: (( (.updatedAt|fromdateiso8601) - (.startedAt|fromdateiso8601) ) // 0)}
    ] | .[0:1]' "$TMP" > "$OUT/run_summary.json"
# Echo auxiliar para logs/diagnóstico
jq -n --argjson count "$(jq 'length' "$OUT/run_summary.json")" '{count:$count}'
