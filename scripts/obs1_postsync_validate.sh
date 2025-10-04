#!/usr/bin/env bash
set -euo pipefail
mkdir -p out/diagnostics
RUSTFLAGS=-Dwarnings cargo check --all-targets --all-features 2>&1 | tee out/diagnostics/check-sync.txt
cargo clean
cargo build -q
cargo test --no-run 2>&1 | tee out/diagnostics/test-norun-sync.txt || true
cargo test -q 2>&1 | tee out/diagnostics/test-run-sync.txt || true
if grep -R "cfg(feature = \"obs\")" -n src tests >/dev/null 2>&1; then
  cargo test --features obs -q 2>&1 | tee -a out/diagnostics/test-run-sync.txt || true
fi
printf '\n[obs1] check-sync.txt (first 60 lines)\n'
sed -n '1,60p' out/diagnostics/check-sync.txt
printf '\n[obs1] test-norun-sync.txt (first 60 lines)\n'
sed -n '1,60p' out/diagnostics/test-norun-sync.txt
printf '\n[obs1] test-run-sync.txt (first 60 lines)\n'
sed -n '1,60p' out/diagnostics/test-run-sync.txt
