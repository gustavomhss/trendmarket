#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF' >&2
Usage: ./ops/scripts/gate_a110.sh [--require-green|--check] [--timeout <seconds>]

Options:
  --require-green    Execute the A110 gate validations (current default behavior).
  --check            Legacy alias for --require-green (maintained for backward compatibility).
  --timeout <secs>   Abort the validations if they exceed the provided timeout in seconds.
  -h, --help         Show this message and exit.
EOF
}

require_green=false
timeout_secs=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --require-green|--check)
      require_green=true
      shift
      ;;
    --timeout)
      if [[ $# -lt 2 ]]; then
        echo "[A110][ERROR] --timeout requires an argument" >&2
        usage
        exit 1
      fi
      timeout_secs="$2"
      if ! [[ $timeout_secs =~ ^[0-9]+$ ]] || (( timeout_secs <= 0 )); then
        echo "[A110][ERROR] --timeout expects a positive integer (seconds)" >&2
        usage
        exit 1
      fi
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "[A110][ERROR] Unsupported argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ $require_green != true ]]; then
  echo "[A110][ERROR] Missing required flag: --require-green (or legacy --check)." >&2
  usage
  exit 1
fi

script_path="$(mktemp)"
trap 'rm -f "$script_path"' EXIT

cat <<'PY' >"$script_path"
import json
from pathlib import Path
import sys

errors = []
watchers_path = Path('ops/watchers/core.yaml')
hooks_candidates = [Path('ops/hooks/a110.yml'), Path('ops/hooks/a110.yaml')]
hooks_path = next((candidate for candidate in hooks_candidates if candidate.exists()), None)

if not watchers_path.exists():
    errors.append(f"missing watchers file: {watchers_path}")
if hooks_path is None:
    errors.append("missing hooks file: ops/hooks/a110.yml (or fallback ops/hooks/a110.yaml)")

if errors:
    print("\n".join(errors))
    sys.exit(1)

with watchers_path.open() as fh:
    watchers_data = json.load(fh)
with hooks_path.open() as fh:
    hooks_raw = json.load(fh)

if isinstance(hooks_raw, dict):
    hooks = hooks_raw.get("hooks", [])
    hooks_required_fields = ["kpi", "threshold", "window", "owner", "rollback", "action"]
    optional_fields = ["playbook", "evidence", "domain"]
elif isinstance(hooks_raw, list):
    hooks = hooks_raw
    hooks_required_fields = ["domain", "kpi", "threshold", "window", "owner", "rollback", "action"]
    optional_fields = ["evidence"]
else:
    errors.append(f"unsupported hooks schema in {hooks_path}")
    hooks = []
    hooks_required_fields = []
    optional_fields = []

expected_domains = ["DEC", "PM", "DATA", "ML", "FE", "SEC/PRIV", "PLAT", "INT"]
domains = watchers_data.get("domains", {})
if sorted(domains.keys()) != sorted(expected_domains):
    errors.append(f"domains mismatch: expected {expected_domains}, found {sorted(domains.keys())}")

watcher_defs = watchers_data.get("watchers", {})
all_domain_watchers = []
for domain, watchers in domains.items():
    if not watchers:
        errors.append(f"domain {domain} has no watchers configured")
    for watcher in watchers:
        all_domain_watchers.append(watcher)
        if watcher not in watcher_defs:
            errors.append(f"watcher {watcher} referenced in domain {domain} but missing definition")

if not hooks:
    errors.append(f"no hooks defined in {hooks_path}")

hooks_by_name = {}
watchers_with_hooks = set()
for hook in hooks:
    name = hook.get("hook")
    if not name:
        errors.append("found hook entry without name")
        continue
    if name in hooks_by_name:
        errors.append(f"duplicate hook name detected: {name}")
    hooks_by_name[name] = hook

    for field in hooks_required_fields:
        if field not in hook or hook[field] in (None, ""):
            errors.append(f"hook {name} missing required field: {field}")
    watcher_list = hook.get("watchers", [])
    if not watcher_list:
        errors.append(f"hook {name} missing watchers binding")
        continue
    for watcher in watcher_list:
        watchers_with_hooks.add(watcher)
        if watcher not in watcher_defs:
            errors.append(f"hook {name} references unknown watcher {watcher}")

    for field in optional_fields:
        if field in hook and hook[field] in (None, ""):
            errors.append(f"hook {name} has empty optional field: {field}")

    playbook = hook.get("playbook")
    if playbook:
        playbook_path = playbook.split('#', 1)[0]
        if playbook_path:
            pb_file = Path(playbook_path)
            if not pb_file.exists():
                errors.append(f"playbook path not found for hook {name}: {playbook_path}")

watchers_missing_hooks = sorted(set(all_domain_watchers) - watchers_with_hooks)
if watchers_missing_hooks:
    errors.append(f"watchers sem hook mapeado: {watchers_missing_hooks}")

for watcher, meta in watcher_defs.items():
    hook_name = meta.get("hook")
    if not hook_name:
        errors.append(f"watcher {watcher} sem hook configurado")
        continue
    if hook_name not in hooks_by_name:
        errors.append(f"watcher {watcher} aponta para hook inexistente {hook_name}")

if errors:
    for err in errors:
        print(f"[A110][ERROR] {err}")
    sys.exit(1)

print("[A110] Gate check passed: watchers, hooks e runbooks consistentes.")
PY

if [[ -n $timeout_secs ]]; then
  if ! command -v timeout >/dev/null 2>&1; then
    echo "[A110][ERROR] --timeout requested but 'timeout' command is not available" >&2
    exit 1
  fi
  timeout "$timeout_secs" python "$script_path"
else
  python "$script_path"
fi
