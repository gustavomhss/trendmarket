#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: $0 --check" >&2
}

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

case "$1" in
  --check)
    python - <<'PY'
import json
from pathlib import Path
import sys

errors = []
watchers_path = Path('ops/watchers/core.yaml')
hooks_path = Path('ops/hooks/a110.yaml')

if not watchers_path.exists():
    errors.append(f"missing watchers file: {watchers_path}")
if not hooks_path.exists():
    errors.append(f"missing hooks file: {hooks_path}")

if errors:
    print("\n".join(errors))
    sys.exit(1)

with watchers_path.open() as fh:
    watchers_data = json.load(fh)
with hooks_path.open() as fh:
    hooks_data = json.load(fh)

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

hooks = hooks_data.get("hooks", [])
if not hooks:
    errors.append("no hooks defined in ops/hooks/a110.yaml")

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
    ;;
  *)
    usage
    exit 1
    ;;
esac
