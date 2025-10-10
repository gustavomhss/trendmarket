# Branch Protection & Governance

The JSON document in this directory mirrors the payload accepted by the GitHub
branch protection API. It requires GitHub CLI (`gh`) with sufficient
permissions. No automation runs this script for you — repository admins must
apply the configuration manually.

## Manual application via `gh`

```bash
# Authenticate with gh auth login beforehand
REPO="org/repository"

# Sync labels
gh label sync --repo "$REPO" .github/labels.yml

# Apply branch protection on main
gh api \
  --method PUT \
  -H "Accept: application/vnd.github+json" \
  "/repos/$REPO/branches/main/protection" \
  --input .github/rulesets/branch-protection.json
```

The protection requires:

- All OBS-3 jobs (`lint-promtool`, `rules-test`, `static-lint`, `schema-validate`, `anti-scans`) to pass.
- Signed commits and linear history.
- Two approving reviews with CODEOWNERS coverage and resolved conversations.
- No force pushes or branch deletions.
