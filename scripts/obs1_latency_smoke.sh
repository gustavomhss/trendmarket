#!/usr/bin/env bash
set -euo pipefail

# OBS-1 latency smoke: executa o teste principal com saída capturada para evidência rápida.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

cargo test --test telemetry_latency_tests wrapper_records_latency_and_labels -- --nocapture
