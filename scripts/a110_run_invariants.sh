#!/usr/bin/env bash
set -euo pipefail
IFS=$' '

umask 022

START_TIME="$(date -u '+%s')"
EXIT_NEXTEST=111
EXIT_CARGO=111
SIGNAL_NAME=""
FINALIZED=0
UNEXPECTED_FAILURE=0

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
  if ! mkdir -p artifacts; then
    log_error 'Unable to create artifacts directory.'
    UNEXPECTED_FAILURE=1
    return 1
  fi
  local targets=(
    'artifacts/junit.xml'
    'artifacts/cargo-test.log'
    'artifacts/exitcodes.env'
    'artifacts/summary.md'
    'artifacts/summary.json'
  )
  local target
  for target in "${targets[@]}"; do
    if ! rm -f "$target" 2>/dev/null && [ -e "$target" ]; then
      log_warn "Failed to remove previous artifact: $target"
    fi
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

write_exitcodes() {
  if ! printf 'EXIT_NEXTEST=%s\nEXIT_CARGO=%s\n' "$EXIT_NEXTEST" "$EXIT_CARGO" > artifacts/exitcodes.env; then
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
    printf '- Timestamp (UTC): %s\n' "$now"
    printf '- Duration: %ss\n' "$duration"
    printf '- Signal: %s\n' "${SIGNAL_NAME:-none}"
    printf '- [P1] cargo nextest run --all --all-features --no-fail-fast: exit %s\n' "$EXIT_NEXTEST"
    printf '- [P2] cargo test --all --all-features -- --nocapture: exit %s\n' "$EXIT_CARGO"
    printf '- Gate should fail: %s\n' "$gate_should_fail"
    printf '\nArtifacts:\n'
    printf '- junit: artifacts/junit.xml\n'
    printf '- cargo log: artifacts/cargo-test.log\n'
  } > artifacts/summary.md; then
    log_error 'Failed to write artifacts/summary.md.'
    UNEXPECTED_FAILURE=1
  fi

  if ! {
    printf '{\n'
    printf '  "timestamp_utc": "%s",\n' "$now"
    printf '  "duration_seconds": %s,\n' "$duration"
    printf '  "signal": "%s",\n' "${SIGNAL_NAME:-none}"
    printf '  "commands": [\n'
    printf '    {"name": "[P1] cargo nextest run", "exit_code": %s, "artifact": "artifacts/junit.xml"},\n' "$EXIT_NEXTEST"
    printf '    {"name": "[P2] cargo test", "exit_code": %s, "artifact": "artifacts/cargo-test.log"}\n' "$EXIT_CARGO"
    printf '  ],\n'
    printf '  "failures": {"p1": %s, "p2": %s, "p3": %s},\n' "$p1_failures" "$p2_failures" "$p3_failures"
    printf '  "gate_should_fail": %s\n' "$gate_should_fail"
    printf '}\n'
  } > artifacts/summary.json; then
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

handle_exit() {
  finalize
}

handle_signal() {
  SIGNAL_NAME="$1"
  log_warn "Caught signal: $SIGNAL_NAME"
  finalize
  exit 0
}

trap 'handle_signal INT' INT
trap 'handle_signal TERM' TERM
trap 'handle_exit' EXIT

log_info 'Preparing environment for invariant run.'
if ! ensure_artifacts; then
  EXIT_NEXTEST=111
  EXIT_CARGO=111
  finalize
  exit 0
fi

set_proptest_defaults

log_info 'Running cargo nextest invariants.'
if cargo nextest run --all --all-features --no-fail-fast --junit-output artifacts/junit.xml; then
  EXIT_NEXTEST=0
  log_info 'cargo nextest run completed successfully.'
else
  EXIT_NEXTEST=$?
  log_error "cargo nextest run failed with exit code ${EXIT_NEXTEST}."
fi

log_info 'Running cargo test (full suite).'
if cargo test --all --all-features -- --nocapture | tee artifacts/cargo-test.log; then
  EXIT_CARGO=0
  log_info 'cargo test completed successfully.'
else
  EXIT_CARGO=$?
  log_error "cargo test failed with exit code ${EXIT_CARGO}."
fi

finalize
exit 0
