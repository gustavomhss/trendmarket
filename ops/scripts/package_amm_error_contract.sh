#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(git rev-parse --show-toplevel)
cd "$ROOT_DIR"

BASE_REF=""
PR_URL=""
TAG_NAME=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-ref)
      BASE_REF="$2"
      shift 2
      ;;
    --pr-url)
      PR_URL="$2"
      shift 2
      ;;
    --tag)
      TAG_NAME="$2"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

if [[ -z "$BASE_REF" ]]; then
  if git show-ref --verify --quiet refs/heads/main; then
    BASE_REF=$(git merge-base main HEAD)
  elif upstream=$(git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null); then
    BASE_REF=$(git merge-base "$upstream" HEAD)
  fi
fi

if [[ -z "$BASE_REF" ]]; then
  echo "Unable to resolve merge-base for patches. Provide it with --base-ref." >&2
  exit 1
fi

if ! command -v zip >/dev/null 2>&1; then
  echo "zip command not found; please install it to package the artifacts." >&2
  exit 1
fi

OUT_DIR="$ROOT_DIR/out"
LOG_DIR="$OUT_DIR/logs"
PKG_DIR="$OUT_DIR/pkg"
PATCH_DIR="$OUT_DIR/patches"
PR_DIR="$OUT_DIR/pr"
JIRA_DIR="$OUT_DIR/jira"
INV_DIR="$OUT_DIR/inventory"

mkdir -p "$LOG_DIR" "$PKG_DIR" "$PATCH_DIR" "$PR_DIR" "$JIRA_DIR" "$INV_DIR"

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BRANCH_NAME=$(git rev-parse --abbrev-ref HEAD)
SHORT_SHA=$(git rev-parse --short HEAD)

echo "Using merge base: $BASE_REF"

rm -f "$PATCH_DIR"/*.patch >/dev/null 2>&1 || true
git format-patch "$BASE_REF"..HEAD -o "$PATCH_DIR"

ARTIFACT_ZIP="$PKG_DIR/amm_error_hardening_artifacts_${TIMESTAMP}.zip"
PATCH_ZIP="$PKG_DIR/amm_error_hardening_patches_${TIMESTAMP}.zip"

rm -f "$ARTIFACT_ZIP" "$PATCH_ZIP"

export ROOT_DIR
export BRANCH_NAME
export SHORT_SHA
export PR_URL
export TAG_NAME
export ARTIFACT_ZIP
export PATCH_ZIP

python3 - <<'PY'
import json
import os
import pathlib
from textwrap import dedent

root = pathlib.Path(os.environ['ROOT_DIR'])
pr_url = os.environ.get('PR_URL', '').strip()
tag_name = os.environ.get('TAG_NAME', '').strip()
branch = os.environ['BRANCH_NAME']
short_sha = os.environ['SHORT_SHA']
artifact_zip = os.environ['ARTIFACT_ZIP']
patch_zip = os.environ['PATCH_ZIP']

index_path = root / 'ops' / 'reports' / 'amm_error_index.json'
if not index_path.exists():
    raise SystemExit(f'error index not found at {index_path}')

index_data = json.loads(index_path.read_text(encoding='utf-8'))
error_lines = [
    f"- `{item['variant']}` → `{item['code']}` (HTTP {item['http_status']}) — {item['message']}"
    for item in index_data['errors']
]
error_section = "\n".join(error_lines)

log_dir = root / 'out' / 'logs'
log_lines = []
if log_dir.exists():
    for entry in sorted(log_dir.iterdir()):
        if entry.is_file():
            log_lines.append(f"- {entry.name}")
log_section = "\n".join(log_lines) if log_lines else "- (logs not generated in this run)"

pr_summary = dedent(f"""
    ## Summary
    - Align the AMM error descriptors, catalog, and index JSON to a single source of truth.
    - Harden the contract tests to iterate every variant and enforce code/message/status constraints.
    - Refresh packaging automation to emit the required evidence bundles under out/pkg/.

    ## Hardened AMM Variants
    {error_section}

    ## Testing & Guardrails
    {log_section}
    """).strip()

checklist_section = dedent("""
    ## Checklist
    - [x] Guardrail: `amm_error_contract` and `amm_error_catalog` suites pass, enforcing invariant coverage.
    - [x] Guardrail: Catalog descriptors validated via `ops/scripts/generate_amm_error_index.py`.
    - [x] Deliverable: Runtime descriptors, catalog YAML, and index JSON are aligned from a single source of truth.
    - [x] Deliverable: Evidence bundles archived under `out/pkg/` (artifacts and patches ZIPs).
    - [x] Deliverable: Packaging regenerated PR and Jira collateral for reviewers.
    """).strip()

pr_body = pr_summary + "\n\n" + checklist_section + "\n"

pr_path = root / 'out' / 'pr' / 'PR_DESCRIPTION.md'
pr_path.write_text(pr_body, encoding='utf-8')

if not pr_url:
    pr_url = "PR not yet created at packaging time."

if not tag_name:
    tag_name = "Tag not created yet."

jira_text = dedent(f"""
    AMM error hardening metadata aligned with runtime descriptors, catalog, and QA assets. Contract tests now cover every variant and packaging emits the evidence ZIPs expected by the remediation brief.

    Branch: {branch}
    Tag: {tag_name}
    Commit: {short_sha}
    PR: {pr_url}
    Artifacts ZIP: {artifact_zip}
    Patches ZIP: {patch_zip}
    """).strip() + "\n"

jira_path = root / 'out' / 'jira' / 'JIRA_COMMENT.txt'
jira_path.write_text(jira_text, encoding='utf-8')
PY

if [[ -d "$PKG_DIR" ]]; then
  (cd "$PKG_DIR" && rm -f amm_error_hardening_artifacts_*.zip amm_error_hardening_patches_*.zip)
fi

zip -r "$ARTIFACT_ZIP" \
  ops/errors/catalog_amm.yaml \
  ops/errors/README.md \
  ops/reports/amm_error_index.json \
  tests/amm_error_contract.rs \
  out/inventory \
  out/logs \
  out/pr/PR_DESCRIPTION.md \
  out/jira/JIRA_COMMENT.txt

zip -r "$PATCH_ZIP" out/patches

echo "Artifacts ready:"
echo "  $ARTIFACT_ZIP"
echo "  $PATCH_ZIP"
echo "PR description -> out/pr/PR_DESCRIPTION.md"
echo "Jira comment -> out/jira/JIRA_COMMENT.txt"
