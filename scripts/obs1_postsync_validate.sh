#!/usr/bin/env bash
set -euo pipefail

mkdir -p out/diagnostics
status=0

run_with_log() {
  local logfile=$1
  shift

  set +e
  "$@" 2>&1 | tee "$logfile"
  local cmd_status=${PIPESTATUS[0]}
  set -e

  if (( cmd_status != 0 )) && (( status == 0 )); then
    status=$cmd_status
  fi
}

run_with_log out/diagnostics/check-sync.txt env RUSTFLAGS=-Dwarnings cargo check --all-targets --all-features
run_with_log out/diagnostics/test-norun-sync.txt cargo test --no-run
run_with_log out/diagnostics/test-run-sync.txt cargo test -q

if rg --files-with-matches 'cfg\(feature = "obs"\)' src tests >/dev/null 2>&1; then
  run_with_log out/diagnostics/test-run-obs-sync.txt cargo test --features obs -q
fi

tail -n 80 out/diagnostics/check-sync.txt || true
tail -n 80 out/diagnostics/test-norun-sync.txt || true
tail -n 80 out/diagnostics/test-run-sync.txt || true
if [[ -f out/diagnostics/test-run-obs-sync.txt ]]; then
  tail -n 80 out/diagnostics/test-run-obs-sync.txt || true
fi

exit $status
