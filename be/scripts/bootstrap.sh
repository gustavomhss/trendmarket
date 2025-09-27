#!/usr/bin/env bash
set -euo pipefail

COMMAND="${1:-}"
DOMAIN_SLUG="be"
OWNER="DEC"
PYTHON_BIN="${PYTHON_BIN:-python3}"
WATCHERS_JSON='["api_breaking_change_watch","metrics_decision_hook_gap_watch","slo_budget_breach_watch","runtime_eol_watch","dep_vuln_watch"]'
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
REPO_ROOT="$(git -C "$ROOT_DIR" rev-parse --show-toplevel)"
INVENTORY="$REPO_ROOT/ops/reports/inventory.json"
BUILD_DIR="$ROOT_DIR/build"
EVIDENCE_DIR="$REPO_ROOT/ops/evidence"

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
    raise SystemExit(f"owner divergente para {slug}: {domain.get('owner')} != {owner}")
registered = set(domain.get("watchers", []))
missing = sorted(set(expected_watchers) - registered)
if missing:
    raise SystemExit(f"watchers ausentes para {slug}: {missing}")
print(f"[inventory] domínio {slug} consistente com owner={owner}")
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
manifest = {
    "domain": slug,
    "owner": "${OWNER}",
    "watchers": json.loads(r'''${WATCHERS_JSON}'''),
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "inventory": "${INVENTORY}"
}
(root / "manifest.json").write_text(json.dumps(manifest, indent=2, ensure_ascii=False))
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
    "source": "be/scripts/bootstrap.sh"
}
path.write_text(json.dumps(record, indent=2, ensure_ascii=False))
print(f"[evidence] publicação registrada em {path}")
PY
}

case "$COMMAND" in
  lint)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
from pathlib import Path
readme = Path("be/README.md")
required = ["Watchers", "Owner", "Makefile"]
text = readme.read_text(encoding="utf-8")
missing = [tok for tok in required if tok.lower() not in text.lower()]
if missing:
    raise SystemExit(f"README incompleto: {missing}")
print("[lint] README documenta owner e watchers")
PY
    ;;
  test)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
from pathlib import Path
makefile = Path("be/Makefile").read_text()
for target in ("lint", "test", "build", "evidence", "hooks.dry", "watchers.dry"):
    if f"{target}:" not in makefile:
        raise SystemExit(f"target {target} ausente no Makefile")
print("[test] Makefile cobre os alvos principais")
PY
    ;;
  build)
    ensure_inventory
    write_manifest
    ;;
  evidence)
    ensure_inventory
    publish_evidence
    ;;
  hooks.dry|watchers.dry)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
print("[hooks/watchers] verificação sintética concluída")
PY
    ;;
  run)
    ensure_inventory
    "$PYTHON_BIN" - <<'PY'
print("[run] ambiente ainda não possui serviço levantado; usar uv/fastapi no commit seguinte")
PY
    ;;
  bootstrap)
    ensure_inventory
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
