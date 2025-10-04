#!/usr/bin/env bash
set -euo pipefail

mkdir -p out/diagnostics

RUSTFLAGS=-Dwarnings cargo check --all-targets --all-features 2>&1 | tee out/diagnostics/check-sync.txt || true
cargo test --no-run 2>&1 | tee out/diagnostics/test-norun-sync.txt || true
cargo test -q 2>&1 | tee out/diagnostics/test-run-sync.txt || true
if rg --quiet 'cfg\(feature = "obs"\)' src tests; then
  cargo test --features obs -q 2>&1 | tee -a out/diagnostics/test-run-sync.txt || true
fi

tail -n 80 out/diagnostics/check-sync.txt || true
tail -n 80 out/diagnostics/test-norun-sync.txt || true
tail -n 80 out/diagnostics/test-run-sync.txt || true
