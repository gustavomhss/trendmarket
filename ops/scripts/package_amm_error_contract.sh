#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR=$(git rev-parse --show-toplevel)
LOG_DIR="$ROOT_DIR/out/logs"
ARTIFACTS_DIR="$ROOT_DIR/out/artifacts"
PATCHES_DIR="$ROOT_DIR/out/patches"

mkdir -p "$LOG_DIR" "$ARTIFACTS_DIR" "$PATCHES_DIR"

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

TEST_LOG="$LOG_DIR/cargo_test.log"
FMT_LOG="$LOG_DIR/rustfmt.log"

echo "Running cargo test (logs -> $TEST_LOG)"
cargo test --all-targets --all-features >"$TEST_LOG" 2>&1

echo "Running rustfmt check (logs -> $FMT_LOG)"
rustfmt --edition 2021 --check \
  src/amm/errors.rs \
  src/bin/obs_demo.rs \
  tests/amm_error_contract.rs >"$FMT_LOG" 2>&1

ARTIFACT_ZIP="$ARTIFACTS_DIR/amm_error_contract_artifacts.zip"
PATCH_FILE="$PATCHES_DIR/amm_error_contract.patch"
PATCH_ZIP="$PATCHES_DIR/amm_error_contract_patches.zip"

rm -f "$ARTIFACT_ZIP" "$PATCH_ZIP"

ZIP_LOG="$ARTIFACTS_DIR/zip_artifacts.log"
PATCH_ZIP_LOG="$PATCHES_DIR/zip_patches.log"

echo "Packaging artifacts -> $ARTIFACT_ZIP"
TMP_ARTIFACT_LOG=$(mktemp)
zip -r "$ARTIFACT_ZIP" \
  ops/errors/catalog_amm.yaml \
  ops/errors/README.md \
  ops/reports/amm_error_index.json \
  tests/amm_error_contract.rs \
  out/logs/ >"$TMP_ARTIFACT_LOG"
mv "$TMP_ARTIFACT_LOG" "$ZIP_LOG"

echo "Writing diff to $PATCH_FILE (base: $BASE_REF)"
TMP_PATCH=$(mktemp)
git diff "$BASE_REF" >"$TMP_PATCH"
mv "$TMP_PATCH" "$PATCH_FILE"

echo "Packaging patches -> $PATCH_ZIP"
TMP_PATCH_LOG=$(mktemp)
zip -j "$PATCH_ZIP" "$PATCH_FILE" >"$TMP_PATCH_LOG"
mv "$TMP_PATCH_LOG" "$PATCH_ZIP_LOG"

echo "Artifacts ready under out/. Files tracked by git exclude the generated ZIP archives."
