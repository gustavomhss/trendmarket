#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

ROOT_DIR=$(git rev-parse --show-toplevel)
OUT_DIR="$ROOT_DIR/out/obs_gatecheck/evidence"
OUT_FILE="$OUT_DIR/obs_policy_scan.json"
TMP_FILE=$(mktemp)
trap 'rm -f "$TMP_FILE"' EXIT

mkdir -p "$OUT_DIR"

record_matches() {
    local kind=$1
    local regex=$2
    shift 2
    local path
    for path in "$@"; do
        if [ -d "$path" ]; then
            while IFS=: read -r file line rest; do
                [ -n "$file" ] || continue
                local snippet
                snippet=$(printf '%s' "$rest" | sed 's/^[[:space:]]*//')
                local rel_path
                rel_path=${file#"$ROOT_DIR/"}
                printf '%s\t%s\t%s\t%s\n' "$kind" "$rel_path" "$line" "$snippet" >>"$TMP_FILE"
            done < <(rg --with-filename --line-number --color=never --no-heading "$regex" "$path" 2>/dev/null || true)
        elif [ -f "$path" ]; then
            while IFS=: read -r file line rest; do
                [ -n "$file" ] || continue
                local snippet
                snippet=$(printf '%s' "$rest" | sed 's/^[[:space:]]*//')
                local rel_path
                rel_path=${file#"$ROOT_DIR/"}
                printf '%s\t%s\t%s\t%s\n' "$kind" "$rel_path" "$line" "$snippet" >>"$TMP_FILE"
            done < <(rg --with-filename --line-number --color=never --no-heading "$regex" "$path" 2>/dev/null || true)
        fi
    done
}

PII_STRING_REGEX="(?i)[\"'](email|cpf|phone|address|name|geo|person_[^\"']*)[\"']\\s*[:=]"
PII_IDENT_REGEX="(?i)\\b(email|cpf|phone|address|geo|person_[a-z0-9_]*)\\b\\s*="
LABEL_REGEX="(?i)[\"'](user_id|account_id|request_id|session_id|[a-z0-9_]*_uuid|[a-z0-9_]*_hash)[\"']"

record_matches "pii" "$PII_STRING_REGEX" "$ROOT_DIR/src" "$ROOT_DIR/docs" "$ROOT_DIR/schemas"
record_matches "pii" "$PII_IDENT_REGEX" "$ROOT_DIR/src"

yaml_files=$(find "$ROOT_DIR" -maxdepth 1 -type f -name '*.yaml')
if [ -n "$yaml_files" ]; then
    for file in $yaml_files; do
        record_matches "pii" "$PII_STRING_REGEX" "$file"
    done
fi

record_matches "labels" "$LABEL_REGEX" "$ROOT_DIR/src" "$ROOT_DIR/docs" "$ROOT_DIR/schemas"
if [ -n "$yaml_files" ]; then
    for file in $yaml_files; do
        record_matches "labels" "$LABEL_REGEX" "$file"
    done
fi

ALLOW_FILE="$ROOT_DIR/.obs_policy_allowlist"
python3 - "$TMP_FILE" "$OUT_FILE" "$ALLOW_FILE" <<'PY'
import json
import os
import sys
from datetime import UTC, datetime

tmp_path, out_path, allow_path = sys.argv[1:4]
allow_rules = []
if os.path.isfile(allow_path):
    with open(allow_path, "r", encoding="utf-8") as handle:
        for raw in handle:
            line = raw.strip()
            if not line or line.startswith("#"):
                continue
            if ":" in line:
                prefix, fragment = line.split(":", 1)
            else:
                prefix, fragment = line, ""
            allow_rules.append((prefix.strip(), fragment.strip()))

def is_allowed(path: str, snippet: str) -> bool:
    for prefix, fragment in allow_rules:
        if prefix and not path.startswith(prefix):
            continue
        if fragment and fragment not in snippet:
            continue
        return True
    return False

def parse_line(raw: str):
    parts = raw.rstrip("\n").split("\t", 3)
    if len(parts) != 4:
        return None
    kind, path, line_no, snippet = parts
    try:
        line_number = int(line_no)
    except ValueError:
        return None
    return {
        "kind": kind,
        "path": path,
        "line": line_number,
        "snippet": snippet,
    }

matches = []
with open(tmp_path, "r", encoding="utf-8") as handle:
    for raw in handle:
        entry = parse_line(raw)
        if not entry:
            continue
        if is_allowed(entry["path"], entry["snippet"]):
            continue
        matches.append(entry)

pii = [m for m in matches if m["kind"] == "pii"]
labels = [m for m in matches if m["kind"] == "labels"]
summary = {
    "pii": len(pii),
    "labels": len(labels),
    "total": len(matches),
}
output = {
    "timestamp": datetime.now(UTC).replace(microsecond=0).isoformat().replace("+00:00", "Z"),
    "summary": summary,
    "matches": {
        "pii": pii,
        "labels": labels,
    },
}
with open(out_path, "w", encoding="utf-8") as handle:
    json.dump(output, handle, ensure_ascii=False, indent=2)

total = summary["total"]
if total:
    sys.exit(1)
PY
