#!/usr/bin/env bash
set -Eeuo pipefail

json_escape() {
  local s="$1"
  s="${s//\\/\\\\}"
  s="${s//\"/\\\"}"
  s="${s//$'\n'/\\n}"
  s="${s//$'\r'/\\r}"
  s="${s//$'\t'/\\t}"
  printf '%s' "$s"
}

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

if [ ! -d "$ROOT" ]; then
  echo '{"error":"repository root not found"}'
  exit 1
fi

if [ -n "${ORR_PROBE_CWD:-}" ]; then
  ROOT="$ORR_PROBE_CWD"
fi

if [ ! -d "$ROOT" ]; then
  echo '{"error":"probe target missing"}'
  exit 1
fi

WRITABLE=false
if [ -w "$ROOT" ]; then
  WRITABLE=true
fi

TOOLS=(bash python3 jq gh)
TOOL_ENTRIES=""
for tool in "${TOOLS[@]}"; do
  if command -v "$tool" >/dev/null 2>&1; then
    AVAILABLE=true
  else
    AVAILABLE=false
  fi
  if [ -n "$TOOL_ENTRIES" ]; then
    TOOL_ENTRIES="$TOOL_ENTRIES,"
  fi
  TOOL_ENTRIES="$TOOL_ENTRIES\"$tool\":$AVAILABLE"
done

printf '{"root":"%s","writable":%s,"tools":{%s}}\n' "$(json_escape "$ROOT")" "$WRITABLE" "$TOOL_ENTRIES"
