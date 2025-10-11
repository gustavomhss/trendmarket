#!/usr/bin/env bash
set -Eeuo pipefail
set +H

SCRIPT_NAME="obs4_branch_bootstrap"
log_ok() {
  local step="$1"
  shift || true
  echo "STEP=${step} OK $*"
}

log_fail() {
  local step="$1"
  shift || true
  echo "STEP=${step} FAIL $*" >&2
}

on_error() {
  local exit_code=$?
  log_fail "${CURRENT_STEP:-unknown}" "Script failed with exit code ${exit_code}" >&2
  exit "$exit_code"
}
trap on_error ERR

CURRENT_STEP="status_check"
if [[ -n "$(git status --porcelain)" ]]; then
  log_fail "status_check" "Working tree not clean. Please stash or commit changes."
  exit 2
fi
log_ok "status_check" "Working tree clean"

CURRENT_STEP="determine_base"
BASE_REF="main"
if git rev-parse --verify origin/main >/dev/null 2>&1; then
  BASE_REF="origin/main"
  log_ok "determine_base" "Using origin/main as base"
else
  log_ok "determine_base" "origin/main not available, using local main"
fi

CURRENT_STEP="checkout_base"
git checkout "${BASE_REF}" >/dev/null 2>&1 || {
  log_fail "checkout_base" "Failed to checkout ${BASE_REF}"
  exit 3
}
log_ok "checkout_base" "Checked out ${BASE_REF}"

CURRENT_STEP="branch_create"
TIMESTAMP="$(date -u +%Y%m%d-%H%M%SZ)"
BRANCH_NAME="obs4/tracing-${TIMESTAMP}"
if git rev-parse --verify "${BRANCH_NAME}" >/dev/null 2>&1; then
  log_fail "branch_create" "Branch ${BRANCH_NAME} already exists"
  exit 3
fi
git checkout -b "${BRANCH_NAME}" >/dev/null
log_ok "branch_create" "Created and switched to ${BRANCH_NAME}"

echo "BRANCH=${BRANCH_NAME}"
log_ok "${SCRIPT_NAME}" "Completed successfully"
