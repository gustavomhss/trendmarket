#!/usr/bin/env sh
# shellcheck disable=SC3040,SC2039
if ! set -Eeuo pipefail 2>/dev/null; then
    set -eu
    if set -o pipefail 2>/dev/null; then
        set -o pipefail
    fi
fi

SCRIPT_DIR=$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd -P)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." >/dev/null 2>&1 && pwd -P)
cd "$REPO_ROOT"

PROMTOOL=${PROMTOOL:-promtool}
YAMLLINT=${YAMLLINT:-yamllint}
SHELLCHECK=${SHELLCHECK:-shellcheck}
RUFF=${RUFF:-ruff}
PYTHON=${PY:-python3}
DEV_CFG=${DEV_CFG:-ops/prometheus/prometheus.dev.yml}
PROD_CFG=${PROD_CFG:-ops/prometheus/prometheus.prod.yml}
RULES_YML=${RULES_YML:-ops/prometheus/rules/core.rules.yml}
TESTS_YML=${TESTS_YML:-ops/prometheus/tests/core.rules.test.yml}
SCHEMA=${SCHEMA:-ops/schemas/manifest.schema.json}
EXAMPLE=${EXAMPLE:-ops/schemas/examples/prom_scrape.example.json}
VERIFY=${VERIFY:-scripts/obs3_verify_manifest.py}
OUTDIR=${OUTDIR:-out/obs_gatecheck}
EVIDIR=${EVIDIR:-${OUTDIR}/evidence}
EVIDENCE_FILE=${EVIDENCE_FILE:-${EVIDIR}/prom_scrape.json}

FAILURES=0
WARNINGS=0

report_ok() {
    printf 'OK   [%s] %s\n' "$1" "$2"
}

report_warn() {
    WARNINGS=$((WARNINGS + 1))
    printf 'WARN [%s] %s\n' "$1" "$2"
}

report_fail() {
    FAILURES=$((FAILURES + 1))
    printf 'FAIL [%s] %s\n' "$1" "$2"
}

ensure_safe_outdir() {
    case "$OUTDIR" in
        out/*) ;;
        *)
            report_fail "safety" "OUTDIR must stay within out/. Current: $OUTDIR"
            exit 1
            ;;
    esac
}

preflight() {
    step="preflight"
    missing=0
    for entry in \
        "promtool|$PROMTOOL|Install promtool from https://prometheus.io/docs/prometheus/latest/installation/" \
        "yamllint|$YAMLLINT|Install yamllint via 'pip install yamllint' or your package manager." \
        "shellcheck|$SHELLCHECK|Install shellcheck from https://github.com/koalaman/shellcheck#installing." \
        "ruff|$RUFF|Install ruff via 'pip install ruff' or your package manager." \
        "python3|$PYTHON|Install Python 3 from https://www.python.org/downloads/ or your package manager."; do
        name=${entry%%|*}
        rest=${entry#*|}
        bin=${rest%%|*}
        hint=${rest#*|}
        if command -v "$bin" >/dev/null 2>&1; then
            version=$($bin --version 2>/dev/null | head -n 1 | tr -d '\r') || version=""
            [ -n "$version" ] || version="version unavailable"
            report_ok "$step" "$name -> $version"
        else
            report_fail "$step" "Missing required tool '$bin'. $hint"
            missing=1
        fi
    done
    if [ "$missing" -ne 0 ]; then
        printf 'Exiting with code 7 due to missing dependencies.\n'
        exit 7
    fi
}

check_placeholders() {
    step="anti-scans/placeholders"
    if output=$(grep -R --line-number --exclude-dir='.git' --exclude='docs/CHANGELOG.md' --exclude='LICENSE' -E 'TODO|FIXME|PLACEHOLDER|CHANGEME|TBD|XXX' . 2>/dev/null); then
        printf '%s\n' "$output"
        report_fail "$step" "Placeholder tokens detected."
    else
        report_ok "$step" "No placeholder tokens detected."
    fi
}

check_conflicts() {
    step="anti-scans/conflicts"
    if output=$(grep -R --line-number --exclude-dir='.git' -E '<<<<<<<|>>>>>>>' . 2>/dev/null); then
        printf '%s\n' "$output"
        report_fail "$step" "Conflict markers detected."
    else
        report_ok "$step" "No conflict markers detected."
    fi
}

lint_yaml() {
    step="yamllint"
    if [ -d "ops/prometheus" ]; then
        if "$YAMLLINT" ops/prometheus; then
            report_ok "$step/config" "ops/prometheus lint passed."
        else
            report_fail "$step/config" "yamllint failed for ops/prometheus."
        fi
    else
        report_warn "$step/config" "ops/prometheus not found; skipping."
    fi
    if [ -f ".github/workflows/obs3-prometheus-ci.yml" ]; then
        if "$YAMLLINT" .github/workflows/obs3-prometheus-ci.yml; then
            report_ok "$step/workflow" "Workflow lint passed."
        else
            report_fail "$step/workflow" "yamllint failed for workflow."
        fi
    else
        report_warn "$step/workflow" "Workflow file missing; skipping."
    fi
}

lint_shell() {
    step="shellcheck"
    set -- scripts/*.sh
    if [ -e "$1" ]; then
        if "$SHELLCHECK" -S warning "$@"; then
            report_ok "$step" "Shell scripts passed shellcheck."
        else
            report_fail "$step" "shellcheck reported issues."
        fi
    else
        report_warn "$step" "No shell scripts found under scripts/."
    fi
}

lint_python() {
    step="ruff"
    set -- scripts/*.py
    if [ -e "$1" ]; then
        if "$RUFF" check scripts; then
            report_ok "$step" "Python lint passed."
        else
            report_fail "$step" "ruff reported issues."
        fi
    else
        report_warn "$step" "No Python files found under scripts/."
    fi
}

check_prometheus_configs() {
    step="promtool-config"
    set --
    if [ -f "$DEV_CFG" ]; then
        set -- "$@" "$DEV_CFG"
    fi
    if [ -f "$PROD_CFG" ]; then
        set -- "$@" "$PROD_CFG"
    fi
    if [ "$#" -gt 0 ]; then
        if "$PROMTOOL" check config "$@"; then
            report_ok "$step" "Prometheus configs valid."
        else
            report_fail "$step" "promtool config check failed."
        fi
    else
        report_warn "$step" "No Prometheus config files found."
    fi
}

check_prometheus_rules() {
    step="promtool-rules"
    if [ -f "$RULES_YML" ]; then
        if "$PROMTOOL" check rules "$RULES_YML"; then
            report_ok "$step" "Prometheus rules valid."
        else
            report_fail "$step" "promtool rules check failed."
        fi
    else
        report_warn "$step" "Rule file $RULES_YML not found."
    fi
}

run_prometheus_tests() {
    step="promtool-tests"
    if [ -f "$TESTS_YML" ]; then
        if "$PROMTOOL" test rules "$TESTS_YML"; then
            report_ok "$step" "Prometheus rule tests passed."
        else
            report_fail "$step" "promtool rule tests failed."
        fi
    else
        report_warn "$step" "Test suite $TESTS_YML not found; skipping."
    fi
}

validate_schema_example() {
    step="schema-example"
    if [ ! -f "$VERIFY" ]; then
        report_fail "$step" "Verifier script $VERIFY not found."
        return 1
    fi
    if [ ! -f "$SCHEMA" ]; then
        report_fail "$step" "Schema file $SCHEMA not found."
        return 1
    fi
    if [ ! -f "$EXAMPLE" ]; then
        report_fail "$step" "Example manifest $EXAMPLE not found."
        return 1
    fi
    if "$PYTHON" "$VERIFY" --manifest "$EXAMPLE" --schema "$SCHEMA"; then
        report_ok "$step" "Example manifest matches schema."
    else
        report_fail "$step" "Schema validation failed for example manifest."
    fi
}

validate_schema_evidence() {
    step="schema-evidence"
    if [ -f "$EVIDENCE_FILE" ]; then
        if "$PYTHON" "$VERIFY" --manifest "$EVIDENCE_FILE" --schema "$SCHEMA"; then
            report_ok "$step" "Evidence manifest matches schema."
        else
            report_fail "$step" "Schema validation failed for evidence manifest."
        fi
    else
        report_warn "$step" "Evidence manifest $EVIDENCE_FILE not found; skipping."
    fi
}

main() {
    ensure_safe_outdir
    preflight
    check_placeholders
    check_conflicts
    lint_yaml
    lint_shell
    lint_python
    check_prometheus_configs
    check_prometheus_rules
    run_prometheus_tests
    validate_schema_example
    validate_schema_evidence
    if [ "$FAILURES" -ne 0 ]; then
        printf 'FAIL [summary] %s failure(s), %s warning(s).\n' "$FAILURES" "$WARNINGS"
        exit 1
    fi
    if [ "$WARNINGS" -ne 0 ]; then
        printf 'WARN [summary] 0 failure(s), %s warning(s).\n' "$WARNINGS"
    else
        printf 'OK   [summary] All mandatory checks passed.\n'
    fi
}

main "$@"
