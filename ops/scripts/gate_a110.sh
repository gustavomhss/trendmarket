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
from collections import defaultdict
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
watcher_domains = defaultdict(set)
all_domain_watchers = []
domain_hook_bindings = {}
for domain, watchers in domains.items():
    if not watchers:
        errors.append(f"domain {domain} has no watchers configured")
    for watcher in watchers:
        all_domain_watchers.append(watcher)
        watcher_domains[watcher].add(domain)
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
    hook_name = meta.get("hook")
    hook_map = meta.get("hooks")

    if hook_map:
        if hook_name:
            errors.append(
                f"watcher {watcher} não deve definir campos 'hook' e 'hooks' simultaneamente"
            )
        if not isinstance(hook_map, dict) or not hook_map:
            errors.append(f"watcher {watcher} possui mapeamento 'hooks' inválido")
            continue

        mapped_domains = {}
        for domain_key, mapped_hook in hook_map.items():
            domain_label = str(domain_key).strip().upper()
            if not domain_label:
                errors.append(
                    f"watcher {watcher} possui domínio inválido no mapeamento de hooks"
                )
                continue

            expected = watcher_domains.get(watcher, set())
            if expected and domain_label not in expected:
                errors.append(
                    f"watcher {watcher} tem hook configurado para domínio inesperado {domain_label}"
                )

            hook_value = str(mapped_hook).strip()
            if not hook_value:
                errors.append(
                    f"watcher {watcher} domínio {domain_label} sem nome de hook definido"
                )
                continue
            if hook_value not in hooks_by_name:
                errors.append(
                    f"watcher {watcher} domínio {domain_label} aponta para hook inexistente {hook_value}"
                )
            mapped_domains[domain_label] = hook_value

        expected_domains = watcher_domains.get(watcher, set())
        missing = expected_domains - set(mapped_domains.keys())
        if missing:
            errors.append(
                f"watcher {watcher} sem hook mapeado para domínio(s) {sorted(missing)}"
            )
        continue

    if not hook_name:
        errors.append(f"watcher {watcher} sem hook configurado")
        continue

    hook_name = str(hook_name).strip()
    if hook_name not in hooks_by_name:
        errors.append(f"watcher {watcher} aponta para hook inexistente {hook_name}")

    expected = watcher_domains.get(watcher, set())
    if len(expected) > 1:
        errors.append(
            f"watcher {watcher} atende domínios {sorted(expected)} mas não possui mapeamento 'hooks'"
        )

if errors:
    for err in errors:
        print(f"[A110][ERROR] {err}", file=sys.stderr)
    sys.exit(1)

print("[A110] Gate check passed: watchers, hooks e runbooks consistentes.")
PY

if [[ -n $timeout_secs ]]; then
  timeout_cmd=""
  for candidate in timeout gtimeout; do
    if command -v "$candidate" >/dev/null 2>&1; then
      timeout_cmd="$candidate"
      break
    fi
  done

  if [[ -z $timeout_cmd ]]; then
    echo "[A110][ERROR] --timeout requested but neither 'timeout' nor 'gtimeout' command is available" >&2
    exit 1
  fi

  "$timeout_cmd" "$timeout_secs" python "$script_path"
else
  python "$script_path"
fi
