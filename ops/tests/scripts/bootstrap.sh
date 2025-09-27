#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-}"
DOMAIN_SLUG="ops-tests"
OWNER="SRE-QA"
PYTHON_BIN="${PYTHON_BIN:-python3}"
WATCHERS_JSON='["metrics_decision_hook_gap_watch","slo_budget_breach_watch","formal_verification_gate_watch"]'
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
REPO_ROOT="$(git -C "$ROOT_DIR" rev-parse --show-toplevel)"
INVENTORY="$REPO_ROOT/ops/reports/inventory.json"
BUILD_DIR="$ROOT_DIR/build"
EVIDENCE_DIR="$REPO_ROOT/ops/evidence"
LOCK_FILE="$ROOT_DIR/uv.lock"

usage() {
  cat <<USAGE
Usage: $(basename "$0") <lint|test|build|run|evidence|hooks.dry|watchers.dry|bootstrap>
USAGE
}

ensure_inventory() {
  "$PYTHON_BIN" - <<PY
import json
from pathlib import Path
slug = "${DOMAIN_SLUG}"
owner = "${OWNER}"
expected_watchers = json.loads(r'''${WATCHERS_JSON}''')
inv_path = Path(r"${INVENTORY}")
if not inv_path.exists():
    raise SystemExit(f"inventário {inv_path} não encontrado")
data = json.loads(inv_path.read_text())
domain = next((item for item in data.get("domains", []) if item.get("slug") == slug), None)
if domain is None:
    raise SystemExit(f"domínio {slug} ausente no inventário")
if domain.get("owner") != owner:
    raise SystemExit(f"owner divergente: {domain.get('owner')} != {owner}")
missing = sorted(set(expected_watchers) - set(domain.get("watchers", [])))
if missing:
    raise SystemExit(f"watchers ausentes para {slug}: {missing}")
print(f"[inventory] domínio {slug} consistente")
PY
}

write_manifest() {
  mkdir -p "$BUILD_DIR"
  "$PYTHON_BIN" - <<PY
import json
from datetime import datetime, timezone
from pathlib import Path
slug = "${DOMAIN_SLUG}"
root = Path(r"${BUILD_DIR}")
man = {
    "domain": slug,
    "owner": "${OWNER}",
    "watchers": json.loads(r'''${WATCHERS_JSON}'''),
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "inventory": "${INVENTORY}",
    "lock_file": "${LOCK_FILE}",
    "probe_suite": []
}
(root / "manifest.json").write_text(json.dumps(man, indent=2, ensure_ascii=False))
print(f"[build] manifest salvo em {root / 'manifest.json'}")
PY
}

publish_evidence() {
  mkdir -p "$EVIDENCE_DIR"
  "$PYTHON_BIN" - <<PY
import json
from datetime import datetime, timezone
from pathlib import Path
slug = "${DOMAIN_SLUG}"
path = Path(r"${EVIDENCE_DIR}") / f"{slug}.json"
record = {
    "domain": slug,
    "owner": "${OWNER}",
    "watchers": json.loads(r'''${WATCHERS_JSON}'''),
    "evidence_type": "inventory-alignment",
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "source": "ops/tests/scripts/bootstrap.sh"
}
path.write_text(json.dumps(record, indent=2, ensure_ascii=False))
print(f"[evidence] publicação registrada em {path}")
PY
}

check_lock() {
  if [[ ! -s "$LOCK_FILE" ]]; then
    echo "lockfile ausente ou vazio: $LOCK_FILE" >&2
    exit 3
  fi
}

case "$COMMAND" in
  lint)
    ensure_inventory
    check_lock
    "$PYTHON_BIN" - <<'PY'
from pathlib import Path
text = Path("ops/tests/README.md").read_text()
if "probes" not in text.lower():
    raise SystemExit("README precisa mencionar probes")
print("[lint] README e lockfile validados")
PY
    ;;
  test)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
from pathlib import Path
makefile = Path("ops/tests/Makefile").read_text()
required = ["lint", "test", "build", "evidence"]
missing = [target for target in required if f"{target}:" not in makefile]
if missing:
    raise SystemExit(f"targets ausentes: {missing}")
print("[test] Makefile cobre os alvos essenciais")
PY
    ;;
  build)
    ensure_inventory
    check_lock
    write_manifest
    ;;
  evidence)
    ensure_inventory
    publish_evidence
    ;;
  hooks.dry|watchers.dry)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
print("[hooks/watchers] simulação concluída para ops/tests")
PY
    ;;
  run)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
print("[run] execute probes sintéticas via make hooks.dry")
PY
    ;;
  bootstrap)
    ensure_inventory
    check_lock
    write_manifest
    publish_evidence
    ;;
  "")
    usage
    exit 1
    ;;
  *)
    echo "comando desconhecido: $COMMAND" >&2
    usage
    exit 2
    ;;
esac
