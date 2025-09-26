#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'
umask 022

START_TIME="$(date -u '+%s')"
EXIT_NEXTEST=111
EXIT_CARGO=111
SIGNAL_NAME=""
FINALIZED=0
UNEXPECTED_FAILURE=0
PYTHON_BIN=""

ARTIFACT_DIR="artifacts"

timestamp() {
  date -u '+%Y-%m-%dT%H:%M:%SZ'
}

log_info() {
  printf '[info] %s %s\n' "$(timestamp)" "$*"
}

log_warn() {
  printf '[warn] %s %s\n' "$(timestamp)" "$*" >&2
}

log_error() {
  printf '[error] %s %s\n' "$(timestamp)" "$*" >&2
}

ensure_artifacts() {
  if ! mkdir -p "${ARTIFACT_DIR}"; then
    log_error 'Unable to create artifacts directory.'
    UNEXPECTED_FAILURE=1
    return 1
  fi

  local targets=(
    "${ARTIFACT_DIR}/junit.xml"
    "${ARTIFACT_DIR}/cargo-test.log"
    "${ARTIFACT_DIR}/exitcodes.env"
    "${ARTIFACT_DIR}/summary.md"
    "${ARTIFACT_DIR}/summary.json"
  )
  local t
  for t in "${targets[@]}"; do
    rm -f "$t" 2>/dev/null || true
  done

  return 0
}

set_proptest_defaults() {
  local defaults=(
    'PROPTEST_CASES:256'
    'PROPTEST_MAX_SHRINK_TIME:2000'
    'PROPTEST_MAX_SHRINK_ITERS:1024'
    'PROPTEST_MAX_GLOBAL_REJECTS:4096'
    'PROPTEST_MAX_LOCAL_REJECTS:1024'
    'PROPTEST_MAX_FLAT_MAP_REJECTS:4096'
  )
  local entry name value
  for entry in "${defaults[@]}"; do
    name="${entry%%:*}"
    value="${entry##*:}"
    if [ -z "${!name-}" ]; then
      printf -v "$name" '%s' "$value"
    fi
    export "$name"
  done
}

resolve_python() {
  if [ -n "${PYTHON_BIN}" ] && command -v "${PYTHON_BIN}" >/dev/null 2>&1; then
    return 0
  fi
  local candidate
  for candidate in python3 python; do
    if command -v "$candidate" >/dev/null 2>&1; then
      PYTHON_BIN="$candidate"
      return 0
    fi
  done
  log_error 'Unable to locate a python interpreter (python3 or python).'
  UNEXPECTED_FAILURE=1
  return 1
}

write_exitcodes() {
  if ! printf 'EXIT_NEXTEST=%s\nEXIT_CARGO=%s\n' "$EXIT_NEXTEST" "$EXIT_CARGO" > "${ARTIFACT_DIR}/exitcodes.env"; then
    log_error 'Failed to write artifacts/exitcodes.env.'
    UNEXPECTED_FAILURE=1
  fi
}

emit_summary() {
  local end_time duration gate_should_fail p1_failures p2_failures p3_failures now

  end_time="$(date -u '+%s')"
  duration=$(( end_time - START_TIME ))
  if [ "$duration" -lt 0 ]; then
    duration=0
  fi

  p1_failures=0
  p2_failures=0
  p3_failures=0

  if [ "$EXIT_NEXTEST" -ne 0 ]; then
    p1_failures=$(( p1_failures + 1 ))
  fi
  if [ "$EXIT_CARGO" -ne 0 ]; then
    p2_failures=$(( p2_failures + 1 ))
  fi

  gate_should_fail='false'
  if [ "$p1_failures" -gt 0 ] || [ "$p2_failures" -gt 0 ]; then
    gate_should_fail='true'
  fi

  now="$(timestamp)"

  if ! {
    printf '# Invariant Run Summary\n\n'
    printf -- '- Timestamp (UTC): %s\n' "$now"
    printf -- '- Duration: %ss\n' "$duration"
    printf -- '- Signal: %s\n' "${SIGNAL_NAME:-none}"
    printf -- '- [P1] cargo nextest run --all --all-features --no-fail-fast --profile ci: exit %s\n' "$EXIT_NEXTEST"
    printf -- '- [P2] cargo test --all --all-features -- --nocapture: exit %s\n' "$EXIT_CARGO"
    printf -- '- Gate should fail: %s\n' "$gate_should_fail"
    printf '\nArtifacts:\n'
    printf -- '- junit: %s/junit.xml\n' "$ARTIFACT_DIR"
    printf -- '- cargo log: %s/cargo-test.log\n' "$ARTIFACT_DIR"
    printf -- '- exit codes: %s/exitcodes.env\n' "$ARTIFACT_DIR"
  } > "${ARTIFACT_DIR}/summary.md"; then
    log_error 'Failed to write artifacts/summary.md.'
    UNEXPECTED_FAILURE=1
  fi

  if ! {
    printf '{\n'
    printf '  "timestamp_utc": "%s",\n' "$now"
    printf '  "duration_seconds": %s,\n' "$duration"
    printf '  "signal": "%s",\n' "${SIGNAL_NAME:-none}"
    printf '  "commands": [\n'
    printf '    {"name": "[P1] cargo nextest run", "exit_code": %s, "artifact": "%s/junit.xml"},\n' "$EXIT_NEXTEST" "$ARTIFACT_DIR"
    printf '    {"name": "[P2] cargo test", "exit_code": %s, "artifact": "%s/cargo-test.log"}\n' "$EXIT_CARGO" "$ARTIFACT_DIR"
    printf '  ],\n'
    printf '  "failures": {"p1": %s, "p2": %s, "p3": %s},\n' "$p1_failures" "$p2_failures" "$p3_failures"
    printf '  "gate_should_fail": %s\n' "$gate_should_fail"
    printf '}\n'
  } > "${ARTIFACT_DIR}/summary.json"; then
    log_error 'Failed to write artifacts/summary.json.'
    UNEXPECTED_FAILURE=1
  fi

  log_info "Total duration: ${duration}s"
  if [ "$gate_should_fail" = 'true' ]; then
    log_warn 'Gate would fail based on recorded exit codes.'
  else
    log_info 'Gate would pass based on recorded exit codes.'
  fi
}

finalize() {
  if [ "$FINALIZED" -eq 1 ]; then
    return
  fi
  FINALIZED=1
  write_exitcodes
  emit_summary
  if [ "$UNEXPECTED_FAILURE" -ne 0 ]; then
    log_warn 'Unexpected issues encountered; check artifacts for details.'
  fi
}

handle_exit() { finalize; }

handle_signal() {
  SIGNAL_NAME="$1"
  log_warn "Caught signal: $SIGNAL_NAME"
  finalize
  exit 0
}

run_with_tee() {
  local log_path="$1"
  shift
  if ! resolve_python; then
    return 100
  fi
  "$PYTHON_BIN" - "$log_path" "$@" <<'PY'
import subprocess
import sys
from typing import List

def main() -> int:
    if len(sys.argv) < 3:
        sys.stderr.write('Usage error: expected log path and command.\n')
        return 97
    log_path = sys.argv[1]
    cmd: List[str] = sys.argv[2:]
    try:
        f = open(log_path, 'w', encoding='utf-8')
    except OSError as exc:
        sys.stderr.write(f'Unable to open log file {log_path}: {exc}\n')
        return 98
    try:
        p = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding='utf-8',
            errors='replace'
        )
    except OSError as exc:
        f.close()
        sys.stderr.write(f'Failed to start command: {exc}\n')
        return 99
    assert p.stdout is not None
    try:
        for chunk in p.stdout:
            sys.stdout.write(chunk)
            sys.stdout.flush()
            f.write(chunk)
        p.stdout.close()
        rc = p.wait()
    finally:
        f.flush()
        f.close()
    return rc

if __name__ == '__main__':
    sys.exit(main())
PY
}

trap 'handle_signal INT'  INT
trap 'handle_signal TERM' TERM
trap 'handle_exit'        EXIT

log_info 'Preparing environment for invariant run.'
if ! ensure_artifacts; then
  EXIT_NEXTEST=111
  EXIT_CARGO=111
  finalize
  exit 0
fi

set_proptest_defaults

log_info 'Running cargo nextest invariants.'
if cargo nextest run --all --all-features --no-fail-fast --profile ci; then
  EXIT_NEXTEST=0
  log_info 'cargo nextest run completed successfully.'
else
  EXIT_NEXTEST=$?
  log_error "cargo nextest run failed with exit code ${EXIT_NEXTEST}."
fi

# Copia JUnit do perfil 'ci' (ou variantes), com fallback para stub.
JUNIT_DEST="${ARTIFACT_DIR}/junit.xml"
JUNIT_CANDIDATES=(
  "target/nextest/ci/junit.xml"
  "target/nextest/default/junit.xml"
)
FOUND_JUNIT=""
for candidate in "${JUNIT_CANDIDATES[@]}"; do
  if [ -f "$candidate" ]; then
    FOUND_JUNIT="$candidate"
    break
  fi
done
if [ -z "${FOUND_JUNIT}" ]; then
  set +e
  FOUND_JUNIT="$(ls target/nextest/*/junit.xml 2>/dev/null | head -n1)"
  set -e
fi
if [ -n "${FOUND_JUNIT}" ] && [ -f "${FOUND_JUNIT}" ]; then
  cp -f "${FOUND_JUNIT}" "${JUNIT_DEST}"
else
  cat > "${JUNIT_DEST}" <<'XML'
<?xml version="1.0" encoding="UTF-8"?>
<testsuite name="cargo-nextest" tests="1" failures="1">
  <testcase classname="cargo-nextest" name="missing-junit">
    <failure message="P1: JUnit report not found">
      cargo nextest did not produce a junit.xml artifact under target/nextest/*/junit.xml
    </failure>
  </testcase>
</testsuite>
XML
fi

log_info 'Running cargo test (full suite).'
if run_with_tee "${ARTIFACT_DIR}/cargo-test.log" cargo test --all --all-features -- --nocapture; then
  EXIT_CARGO=0
  log_info 'cargo test completed successfully.'
else
  EXIT_CARGO=$?
  log_error "cargo test failed with exit code ${EXIT_CARGO}."
fi

finalize
exit 0
