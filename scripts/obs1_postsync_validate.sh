#!/usr/bin/env bash
set -euo pipefail

mkdir -p out/diagnostics
status=0

RUSTFLAGS=-Dwarnings cargo check --all-targets --all-features 2>&1 | tee out/diagnostics/check-sync.txt || status=$?
cargo clean || status=$?
cargo build -q || status=$?
cargo test --no-run 2>&1 | tee out/diagnostics/test-norun-sync.txt || status=$?
cargo test -q 2>&1 | tee out/diagnostics/test-run-sync.txt || status=$?
if rg --glob '*.rs' --quiet 'cfg\(feature = "obs"\)' src tests; then
  cargo test --features obs -q 2>&1 | tee -a out/diagnostics/test-run-sync.txt || status=$?
fi

tail -n 80 out/diagnostics/check-sync.txt || true
tail -n 80 out/diagnostics/test-norun-sync.txt || true
tail -n 80 out/diagnostics/test-run-sync.txt || true

exit $status
