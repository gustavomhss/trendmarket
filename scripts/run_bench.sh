#!/usr/bin/env bash
set -euo pipefail


# Otimizações típicas para stage/local
export RUSTFLAGS="-C target-cpu=native"


# 1) Compila e roda APENAS os benches Criterion deste crate
cargo bench -p credit-engine-core --bench bench_swap
cargo bench -p credit-engine-core --bench bench_liquidity


# 2) Gera relatório p95 (falha se p95 ≥ limiar; default 50 µs)
python3 scripts/bench_report.py
