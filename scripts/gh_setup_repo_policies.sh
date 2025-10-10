#!/usr/bin/env bash
# Optional helper script to apply GitHub governance artifacts (labels + branch protection).
# Requires GitHub CLI (gh) authenticated with write permissions to the target repository.
set -euo pipefail

usage() {
  cat <<USAGE
Usage: $0 <owner/repo>

Idempotently applies labels and branch protection rules defined in .github.
This script is optional and must be executed manually by repository admins.
USAGE
}

if [[ $# -ne 1 ]]; then
  usage
  exit 1
fi

REPO="$1"
ROOT=$(git rev-parse --show-toplevel)

if ! command -v gh >/dev/null 2>&1; then
  echo "GitHub CLI (gh) is required" >&2
  exit 1
fi

echo "Applying labels to $REPO"
gh label sync --repo "$REPO" "$ROOT/.github/labels.yml"

echo "Applying branch protection to main"
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "/repos/$REPO/branches/main/protection" \
  --input "$ROOT/.github/rulesets/branch-protection.json"

echo "Done. Review GitHub settings to confirm enforcement."
