#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR=$(git rev-parse --show-toplevel)
STATUS=()
FAIL=0

log_step() {
  printf '\n[%s] %s\n' "$(date --iso-8601=seconds)" "$1"
}

run_check() {
  local name="$1"
  shift
  log_step "Running ${name}"
  if "$@"; then
    STATUS+=("G|${name}")
  else
    STATUS+=("R|${name}")
    FAIL=1
  fi
}

run_check "yamllint" yamllint ops/prometheus ops/schemas .github/workflows
run_check "ruff" ruff check scripts ops/prometheus
run_check "shellcheck" shellcheck scripts/obs_t3_prom_scrape_run.sh scripts/obs3_all_checks.sh scripts/gh_setup_repo_policies.sh
run_check "promtool check config (dev)" promtool check config ops/prometheus/prometheus.dev.yml
run_check "promtool check config (prod)" promtool check config ops/prometheus/prometheus.prod.yml
run_check "promtool test rules" promtool test rules ops/prometheus/tests/core.rules.test.yml
run_check "manifest schema" python3 "$ROOT_DIR/scripts/obs3_verify_manifest.py" --schema "$ROOT_DIR/ops/schemas/manifest.schema.json" --manifest "$ROOT_DIR/ops/schemas/examples/prom_scrape.example.json"

printf '\nOBS-3 all-checks summary (RAG)\n'
for item in "${STATUS[@]}"; do
  IFS='|' read -r color label <<<"$item"
  case "$color" in
    G) printf '  [GREEN] %s\n' "$label" ;;
    A) printf '  [AMBER] %s\n' "$label" ;;
    R) printf '  [RED] %s\n' "$label" ;;
  esac
done

exit $FAIL
