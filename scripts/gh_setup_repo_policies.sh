#!/usr/bin/env sh
set -Eeuo pipefail

STRICT=0
DRY_RUN=0
DEFAULT_BRANCH=""
PERMISSION_DENIED=0
LABEL_FAILURE=0
PROTECTION_FAILURE=0

usage() {
  cat <<'USAGE'
Usage: gh_setup_repo_policies.sh [options]

Options:
  --default-branch <name>  Override detected default branch
  --dry-run                 Print intended operations without applying changes (exit 10)
  --strict                  Exit with non-zero code when lacking permissions
  -h, --help                Show this message

Exit codes:
  0  Success (or permissions missing in non-strict mode)
  7  Required dependency missing
  8  Failed to apply labels
  9  Failed to apply branch protection or strict permission failure
 10  Dry-run completed without changes
USAGE
}

log() {
  printf '%s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

error() {
  printf 'error: %s\n' "$*" >&2
}

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    error "Required dependency '$1' not found in PATH."
    case "$1" in
      gh)
        warn "Install GitHub CLI: https://cli.github.com/"
        ;;
      python3)
        warn "Install Python 3 to parse YAML/JSON payloads."
        ;;
    esac
    exit 7
  fi
}

parse_args() {
  while [ $# -gt 0 ]; do
    case "$1" in
      --strict)
        STRICT=1
        ;;
      --dry-run)
        DRY_RUN=1
        ;;
      --default-branch)
        if [ $# -lt 2 ]; then
          error "--default-branch requires an argument"
          usage
          exit 64
        fi
        DEFAULT_BRANCH="$2"
        shift
        ;;
      -h|--help)
        usage
        exit 0
        ;;
      *)
        error "Unknown option: $1"
        usage
        exit 64
        ;;
    esac
    shift
  done
}

parse_labels_catalog() {
  python3 - "$1" <<'PY'
import pathlib, sys
path = pathlib.Path(sys.argv[1])
if not path.exists():
    sys.exit(1)
items = []
current = {}
for line in path.read_text().splitlines():
    stripped = line.strip()
    if stripped.startswith('- name:'):
        if current:
            items.append(current)
            current = {}
        current['name'] = stripped.split(':', 1)[1].strip().strip('"')
    elif stripped.startswith('color:'):
        current['color'] = stripped.split(':', 1)[1].strip().strip('"')
    elif stripped.startswith('description:'):
        current['description'] = stripped.split(':', 1)[1].strip()
if current:
    items.append(current)
for item in items:
    name = item.get('name')
    color = item.get('color')
    description = item.get('description', '')
    if not name or not color:
        continue
    print(f"{name}|{color}|{description}")
PY
}

run_gh() {
  LAST_ERROR_PERMISSION=0
  description="$1"
  shift
  if [ "$DRY_RUN" -eq 1 ]; then
    log "[dry-run] $description: gh $*"
    return 0
  fi
  if output=$(gh "$@" 2>&1); then
    if [ -n "$output" ]; then
      printf '%s\n' "$output"
    fi
    return 0
  fi
  status=$?
  printf '%s\n' "$output" >&2
  if printf '%s' "$output" | grep -Ei 'forbidden|insufficient|must have admin|resource not accessible|requires authentication' >/dev/null 2>&1; then
    LAST_ERROR_PERMISSION=1
    warn "Skipping due to insufficient permissions: $description"
    return 1
  fi
  return "$status"
}

apply_labels() {
  catalog="$1"
  if [ ! -f "$catalog" ]; then
    log "Label catalog '$catalog' not found; skipping labels."
    return 0
  fi
  log "Applying labels from $catalog"
  if ! LABEL_LINES=$(parse_labels_catalog "$catalog" 2>/dev/null); then
    error "Failed to parse label catalog."
    LABEL_FAILURE=1
    return 1
  fi
  OLDIFS=$IFS
  IFS='\n'
  for entry in $LABEL_LINES; do
    IFS='|' read -r name color description <<EOF
$entry
EOF
    IFS='\n'
    if [ -z "$name" ]; then
      continue
    fi
    hex_color="${color#\#}"
    if LABEL_INFO=$(gh label view "$name" --json name,color,description --jq '[(.color // ""), (.description // "")] | join("|")' 2>&1); then
      current_color=$(printf '%s' "$LABEL_INFO" | cut -d'|' -f1)
      current_desc=$(printf '%s' "$LABEL_INFO" | cut -d'|' -f2-)
      if [ "$current_color" = "$hex_color" ] && [ "$current_desc" = "$description" ]; then
        log "Label '$name' already up to date."
        continue
      fi
      if ! run_gh "update label $name" label edit "$name" --color "$hex_color" --description "$description"; then
        if [ "$LAST_ERROR_PERMISSION" -eq 1 ]; then
          PERMISSION_DENIED=1
          continue
        fi
        LABEL_FAILURE=1
      fi
    else
      if printf '%s' "$LABEL_INFO" | grep -Ei 'not found|could not resolve' >/dev/null 2>&1; then
        if ! run_gh "create label $name" label create "$name" --color "$hex_color" --description "$description"; then
          if [ "$LAST_ERROR_PERMISSION" -eq 1 ]; then
            PERMISSION_DENIED=1
            continue
          fi
          LABEL_FAILURE=1
        fi
      elif printf '%s' "$LABEL_INFO" | grep -Ei 'forbidden|insufficient|must have admin|resource not accessible' >/dev/null 2>&1; then
        PERMISSION_DENIED=1
        warn "Skipping label '$name' due to insufficient permissions."
        continue
      else
        printf '%s\n' "$LABEL_INFO" >&2
        LABEL_FAILURE=1
      fi
    fi
  done
  IFS=$OLDIFS
}

build_ruleset_payload() {
  python3 - "$1" "$2" <<'PY'
import json, pathlib, sys
ruleset_path = pathlib.Path(sys.argv[1])
default_branch = sys.argv[2] or None
payload = json.loads(ruleset_path.read_text())
refs = payload.setdefault('conditions', {}).setdefault('ref_name', {}).setdefault('include', [])
if default_branch:
    default_ref = f"refs/heads/{default_branch}"
    if default_ref not in refs:
        refs.insert(0, default_ref)
if 'refs/heads/release/*' not in refs:
    refs.append('refs/heads/release/*')
print(json.dumps(payload))
PY
}

ruleset_name_from_payload() {
  python3 - <<'PY'
import json, sys
print(json.loads(sys.stdin.read()).get('name', ''))
PY
}

extract_existing_ruleset_id() {
  python3 - "$1" <<'PY'
import json, sys
name = sys.argv[1]
try:
    data = json.loads(sys.stdin.read())
except json.JSONDecodeError:
    sys.exit(0)
for item in data:
    if item.get('name') == name:
        print(item.get('id', ''))
        break
PY
}

build_fallback_payload() {
  python3 - <<'PY'
import json, sys
checks = [line for line in sys.stdin.read().splitlines() if line]
print(json.dumps({
    "required_status_checks": {
        "strict": True,
        "contexts": checks
    },
    "enforce_admins": True,
    "required_pull_request_reviews": {
        "dismiss_stale_reviews": True,
        "require_code_owner_reviews": True,
        "required_approving_review_count": 2
    },
    "allow_force_pushes": False,
    "allow_deletions": False,
    "required_conversation_resolution": True,
    "required_linear_history": True
}))
PY
}

apply_branch_protection() {
  ruleset_file="$1"
  owner_repo="$2"
  if [ ! -f "$ruleset_file" ]; then
    log "Ruleset model '$ruleset_file' not found; skipping protection."
    return 0
  fi

  if [ -z "$DEFAULT_BRANCH" ]; then
    if DEFAULT_BRANCH=$(git remote show origin 2>/dev/null | awk '/HEAD branch/ {print $NF}' | tail -n1); then
      log "Detected default branch: $DEFAULT_BRANCH"
    else
      error "Unable to determine default branch. Use --default-branch to specify it."
      PROTECTION_FAILURE=1
      return 1
    fi
  else
    log "Using provided default branch: $DEFAULT_BRANCH"
  fi

  PAYLOAD=$(build_ruleset_payload "$ruleset_file" "$DEFAULT_BRANCH")
  if [ -z "$PAYLOAD" ]; then
    error "Failed to build ruleset payload."
    PROTECTION_FAILURE=1
    return 1
  fi
  RULESET_NAME=$(printf '%s' "$PAYLOAD" | ruleset_name_from_payload)

  rulesets_endpoint="repos/$owner_repo/rulesets"
  if rulesets_json=$(gh api -H "Accept: application/vnd.github+json" "$rulesets_endpoint" 2>&1); then
    EXISTING_ID=$(printf '%s' "$rulesets_json" | extract_existing_ruleset_id "$RULESET_NAME")
    if [ -n "$EXISTING_ID" ]; then
      if ! run_gh "update ruleset" api -X PATCH -H "Accept: application/vnd.github+json" "$rulesets_endpoint/$EXISTING_ID" --input - <<EOF
$PAYLOAD
EOF
      then
        if [ "$LAST_ERROR_PERMISSION" -eq 1 ]; then
          PERMISSION_DENIED=1
          return 0
        fi
        PROTECTION_FAILURE=1
        return 1
      fi
    else
      if ! run_gh "create ruleset" api -X POST -H "Accept: application/vnd.github+json" "$rulesets_endpoint" --input - <<EOF
$PAYLOAD
EOF
      then
        if [ "$LAST_ERROR_PERMISSION" -eq 1 ]; then
          PERMISSION_DENIED=1
          return 0
        fi
        PROTECTION_FAILURE=1
        return 1
      fi
    fi
    return 0
  else
    output="$rulesets_json"
    if printf '%s' "$output" | grep -Ei '404|not found|unsupported|preview' >/dev/null 2>&1; then
      warn "Ruleset endpoint unavailable; falling back to classic branch protection."
    elif printf '%s' "$output" | grep -Ei 'forbidden|insufficient|must have admin|resource not accessible' >/dev/null 2>&1; then
      PERMISSION_DENIED=1
      warn "Insufficient permissions to manage rulesets; skipping protection."
      return 0
    else
      printf '%s\n' "$output" >&2
      PROTECTION_FAILURE=1
      return 1
    fi
  fi

  checks=$(printf '%s\n' lint-promtool rules-test static-lint schema-validate anti-scans)
  FALLBACK_PAYLOAD=$(printf '%s' "$checks" | build_fallback_payload)
  if [ "$DRY_RUN" -eq 1 ]; then
    log "[dry-run] Would apply branch protection via legacy endpoint on $DEFAULT_BRANCH"
    return 0
  fi
  if ! run_gh "configure legacy branch protection" api -X PUT -H "Accept: application/vnd.github+json" "repos/$owner_repo/branches/$DEFAULT_BRANCH/protection" --input - <<EOF
$FALLBACK_PAYLOAD
EOF
  then
    if [ "$LAST_ERROR_PERMISSION" -eq 1 ]; then
      PERMISSION_DENIED=1
      return 0
    fi
    PROTECTION_FAILURE=1
    return 1
  fi
  if ! run_gh "enforce signed commits" api -X POST -H "Accept: application/vnd.github+json" "repos/$owner_repo/branches/$DEFAULT_BRANCH/protection/required_signatures"; then
    if [ "$LAST_ERROR_PERMISSION" -eq 1 ]; then
      PERMISSION_DENIED=1
    else
      warn "Could not enforce signed commits via legacy endpoint; check permissions."
    fi
  fi
}

main() {
  parse_args "$@"
  require_tool gh
  require_tool python3

  if ! OWNER_REPO=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null); then
    warn "Unable to determine repository via gh repo view."
    PERMISSION_DENIED=1
    if [ "$STRICT" -eq 1 ]; then
      exit 9
    fi
    exit 0
  fi

  log "Repository: $OWNER_REPO"

  apply_labels ".github/labels.yml"
  apply_branch_protection ".github/rulesets/branch-protection.json" "$OWNER_REPO"

  if [ "$DRY_RUN" -eq 1 ] && [ "$LABEL_FAILURE" -eq 0 ] && [ "$PROTECTION_FAILURE" -eq 0 ] && [ "$PERMISSION_DENIED" -eq 0 ]; then
    log "Dry-run completed. No changes applied."
    exit 10
  fi

  if [ "$PROTECTION_FAILURE" -ne 0 ]; then
    exit 9
  fi
  if [ "$LABEL_FAILURE" -ne 0 ]; then
    exit 8
  fi
  if [ "$PERMISSION_DENIED" -ne 0 ]; then
    warn "Some operations were skipped due to insufficient permissions."
    if [ "$STRICT" -eq 1 ]; then
      exit 9
    fi
  fi

  exit 0
}

main "$@"
