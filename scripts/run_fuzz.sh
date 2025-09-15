#!/usr/bin/env bash
set -euo pipefail
# Executa 10k casos (config no próprio teste)
cargo test --test fuzz_invariants -- --nocapture
