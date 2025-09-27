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
hooks_path = Path('ops/hooks/a110.yml')

if not watchers_path.exists():
    errors.append(f"missing watchers file: {watchers_path}")
if not hooks_path.exists():
    errors.append(f"missing hooks file: {hooks_path}")

if errors:
    for err in errors:
        print(f"[A110][ERROR] {err}", file=sys.stderr)
    sys.exit(1)

def load_json(path: Path) -> dict:
    try:
        with path.open() as fh:
            return json.load(fh)
    except json.JSONDecodeError as exc:
        print(f"[A110][ERROR] failed to parse {path}: {exc}", file=sys.stderr)
        sys.exit(1)


watchers_data = load_json(watchers_path)
hooks_data = load_json(hooks_path)

expected_domains = ["DEC", "PM", "DATA", "ML", "FE", "SEC/PRIV", "PLAT", "INT"]
domains = watchers_data.get("domains", {})
if sorted(domains.keys()) != sorted(expected_domains):
    errors.append(f"domains mismatch: expected {expected_domains}, found {sorted(domains.keys())}")

watcher_defs = watchers_data.get("watchers", {})
all_domain_watchers = []
domain_hook_bindings = {}
for domain, watchers in domains.items():
    if not watchers:
        errors.append(f"domain {domain} has no watchers configured")
    for watcher in watchers:
        all_domain_watchers.append(watcher)
        if watcher not in watcher_defs:
            errors.append(f"watcher {watcher} referenced in domain {domain} but missing definition")
            continue

        hooks_mapping = watcher_defs[watcher].get("hooks")
        if not isinstance(hooks_mapping, dict) or not hooks_mapping:
            errors.append(f"watcher {watcher} lacks per-domain hook mappings")
            continue

        hook_name = hooks_mapping.get(domain)
        if hook_name is None:
            available = ", ".join(sorted(hooks_mapping)) or "<none>"
            errors.append(
                f"watcher {watcher} has no hook assignment for domain {domain} (available: {available})"
            )
            continue

        hook_value = str(hook_name).strip()
        if not hook_value:
            errors.append(f"watcher {watcher} has blank hook assignment for domain {domain}")
            continue

        domain_hook_bindings.setdefault(domain, {})[watcher] = hook_value

hooks = hooks_data.get("hooks", [])
if not hooks:
    errors.append("no hooks defined in ops/hooks/a110.yml")

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

    for field in ("kpi", "threshold", "window", "owner", "rollback", "playbook"):
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

    playbook = hook.get("playbook", "")
    playbook_path = playbook.split('#', 1)[0]
    if playbook_path:
        pb_file = Path(playbook_path)
        if not pb_file.exists():
            errors.append(f"playbook path not found for hook {name}: {playbook_path}")
    else:
        errors.append(f"hook {name} missing playbook path")

watchers_missing_hooks = sorted(set(all_domain_watchers) - watchers_with_hooks)
if watchers_missing_hooks:
    errors.append(f"watchers sem hook mapeado: {watchers_missing_hooks}")

for watcher, meta in watcher_defs.items():
    hooks_mapping = meta.get("hooks")
    if not isinstance(hooks_mapping, dict) or not hooks_mapping:
        errors.append(f"watcher {watcher} missing hooks mapping")
        continue

    for domain_key, hook_name in hooks_mapping.items():
        domain_name = str(domain_key).strip()
        if domain_name and domain_name not in expected_domains:
            errors.append(f"watcher {watcher} references unknown domain {domain_name} in hooks mapping")
        hook_value = str(hook_name).strip()
        if not hook_value:
            errors.append(f"watcher {watcher} has empty hook binding for domain {domain_name or domain_key}")
            continue
        if hook_value not in hooks_by_name:
            errors.append(f"watcher {watcher} aponta para hook inexistente {hook_value}")

for domain, bindings in domain_hook_bindings.items():
    for watcher, hook_name in bindings.items():
        if hook_name not in hooks_by_name:
            errors.append(
                f"watcher {watcher} in domain {domain} aponta para hook inexistente {hook_name}"
            )

if errors:
    for err in errors:
        print(f"[A110][ERROR] {err}", file=sys.stderr)
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
