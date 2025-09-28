#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(git rev-parse --show-toplevel)
LOG_DIR="$ROOT_DIR/out/logs"
PKG_DIR="$ROOT_DIR/out/pkg"
PR_DIR="$ROOT_DIR/out/pr"
JIRA_DIR="$ROOT_DIR/out/jira"

mkdir -p "$LOG_DIR" "$PKG_DIR" "$PR_DIR" "$JIRA_DIR"

clean_dir() {
  local dir="$1"
  if [[ -d "$dir" ]]; then
    find "$dir" -type f ! -name '.gitkeep' -delete 2>/dev/null || true
  fi
}

clean_dir "$LOG_DIR"
clean_dir "$PKG_DIR"
clean_dir "$PR_DIR"
clean_dir "$JIRA_DIR"

if ! command -v zip >/dev/null 2>&1; then
  echo "zip command not found; please install it to package the artifacts." >&2
  exit 1
fi

BASE_REF="${1:-}"
if [[ -z "$BASE_REF" ]]; then
  if git show-ref --verify --quiet refs/heads/main; then
    BASE_REF=$(git merge-base main HEAD)
  elif upstream=$(git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null); then
    BASE_REF=$(git merge-base "$upstream" HEAD)
  fi
fi

if [[ -z "$BASE_REF" ]]; then
  echo "Unable to resolve merge-base for patches. Provide it explicitly as the first argument." >&2
  exit 1
fi

echo "Using merge base: $BASE_REF"

FMT_LOG="$LOG_DIR/cargo_fmt.txt"
CLIPPY_LOG="$LOG_DIR/cargo_clippy.txt"
CHECK_LOG="$LOG_DIR/cargo_check.txt"
TEST_LOG="$LOG_DIR/cargo_test.txt"
RG_TODO_LOG="$LOG_DIR/rg_todo_guardrail.txt"
RG_UNWRAP_LOG="$LOG_DIR/rg_unwrap_guardrail.txt"

failure=0

run_and_capture() {
  local log_file="$1"
  shift
  {
    printf '$ %s\n' "$*"
    "$@"
  } >"$log_file" 2>&1 || {
    local status=$?
    printf '\n[exit-status] %s\n' "$status" >>"$log_file"
    return "$status"
  }
}

run_and_capture "$FMT_LOG" cargo fmt --all -- --check || failure=1
run_and_capture "$CLIPPY_LOG" cargo clippy -- -D warnings || failure=1
run_and_capture "$CHECK_LOG" cargo check || failure=1
run_and_capture "$TEST_LOG" cargo test || failure=1

echo "Running guardrail ripgrep (logs -> $RG_TODO_LOG, $RG_UNWRAP_LOG)"
{
  echo "$ rg --color never --line-number --glob '*.*' 'TODO'"
  rg --color never --line-number --glob '*.*' 'TODO'
} >"$RG_TODO_LOG" 2>&1 || failure=1

{
  echo "$ rg --color never --line-number --glob '*.rs' 'unwrap\('"
  rg --color never --line-number --glob '*.rs' 'unwrap\(' 
} >"$RG_UNWRAP_LOG" 2>&1 || failure=1

timestamp=$(date -u +"%Y%m%dT%H%M%SZ")

ARTIFACT_ZIP="$PKG_DIR/amm_error_contract_artifacts_${timestamp}.zip"
PATCH_FILE="$PR_DIR/amm_error_contract.patch"
PATCH_ZIP="$PR_DIR/amm_error_contract_patches_${timestamp}.zip"
PR_DESCRIPTION="$PR_DIR/PR_DESCRIPTION.md"
JIRA_COMMENT="$JIRA_DIR/JIRA_COMMENT.txt"

ZIP_LOG="$PKG_DIR/zip_artifacts_${timestamp}.txt"
PATCH_ZIP_LOG="$PR_DIR/zip_patches_${timestamp}.txt"
ARTIFACT_SHA="$PKG_DIR/amm_error_contract_artifacts_${timestamp}.sha256"
PATCH_ZIP_SHA="$PR_DIR/amm_error_contract_patches_${timestamp}.sha256"
MANIFEST_FILE="$PKG_DIR/amm_error_contract_manifest_${timestamp}.json"
DIFF_SUMMARY="$PR_DIR/DIFF_SUMMARY.md"

echo "Packaging artifacts -> $ARTIFACT_ZIP"
TMP_ARTIFACT_LOG=$(mktemp)
zip -r "$ARTIFACT_ZIP" \
  ops/errors/catalog_amm.yaml \
  ops/errors/README.md \
  ops/reports/amm_error_index.json \
  tests/amm_error_contract.rs \
  out/logs/ >"$TMP_ARTIFACT_LOG"
mv "$TMP_ARTIFACT_LOG" "$ZIP_LOG"

echo "Writing diff to $PATCH_FILE (base: $BASE_REF, excluding generated out/ contents)"
TMP_PATCH=$(mktemp)
git diff "$BASE_REF" -- . ':(exclude)out/**' >"$TMP_PATCH"
mv "$TMP_PATCH" "$PATCH_FILE"

echo "Packaging patches -> $PATCH_ZIP"
TMP_PATCH_LOG=$(mktemp)
zip -j "$PATCH_ZIP" "$PATCH_FILE" >"$TMP_PATCH_LOG"
mv "$TMP_PATCH_LOG" "$PATCH_ZIP_LOG"
sha256sum "$ARTIFACT_ZIP" | awk '{print $1}' >"$ARTIFACT_SHA"
sha256sum "$PATCH_ZIP" | awk '{print $1}' >"$PATCH_ZIP_SHA"

echo "Writing manifest -> $MANIFEST_FILE"
cat >"$MANIFEST_FILE" <<EOF_MANIFEST
{
  "generated_at": "${timestamp}",
  "artifact_zip": {
    "path": "out/pkg/$(basename "$ARTIFACT_ZIP")",
    "sha256": "$(cat "$ARTIFACT_SHA")"
  },
  "patch_zip": {
    "path": "out/pr/$(basename "$PATCH_ZIP")",
    "sha256": "$(cat "$PATCH_ZIP_SHA")"
  },
  "logs": "out/logs",
  "diff_file": "out/pr/$(basename "$PATCH_FILE")"
}
EOF_MANIFEST

echo "Summarizing diff -> $DIFF_SUMMARY"
{
  echo "# Diff Summary"
  echo
  echo "Generated at ${timestamp} UTC from base ${BASE_REF}."
  echo
  git diff --stat "$BASE_REF" -- . ':(exclude)out/**'
} >"$DIFF_SUMMARY"

echo "Writing PR description -> $PR_DESCRIPTION"
cat >"$PR_DESCRIPTION" <<EOF_PR
# AMM Error Contract Packaging

- Artifacts bundle: $(basename "$ARTIFACT_ZIP")
- Patch bundle: $(basename "$PATCH_ZIP")
- SHA256 (artifacts): $(cat "$ARTIFACT_SHA")
- SHA256 (patches): $(cat "$PATCH_ZIP_SHA")
- Logs directory: out/logs
- Generated at: ${timestamp} UTC

Artifacts capture the contract catalog, maintainer guide, generated index,
regression tests, and guardrail logs. Patch bundle includes the git diff from
$BASE_REF to the current working tree state.
EOF_PR

echo "Writing Jira comment -> $JIRA_COMMENT"
cat >"$JIRA_COMMENT" <<EOF_JIRA
Artifacts ZIP: out/pkg/$(basename "$ARTIFACT_ZIP") (sha256: $(cat "$ARTIFACT_SHA"))
Patches ZIP: out/pr/$(basename "$PATCH_ZIP") (sha256: $(cat "$PATCH_ZIP_SHA"))
Diff summary: out/pr/$(basename "$DIFF_SUMMARY")
Generated at: ${timestamp} UTC

Guardrail logs refreshed via cargo fmt, cargo clippy, cargo check, cargo test,
and ripgrep guardrails (TODO + unwrap).
EOF_JIRA

echo "Artifacts ready under out/."

if [[ "$failure" -ne 0 ]]; then
  echo "One or more guardrail commands failed. See logs under out/logs." >&2
fi
