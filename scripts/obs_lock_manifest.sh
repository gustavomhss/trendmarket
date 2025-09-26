#!/usr/bin/env bash
set -euo pipefail


# Atualiza e reforça pinos-alvo
cargo update -p opentelemetry --precise 0.30.0 || true
cargo update -p opentelemetry_sdk --precise 0.30.0 || true
cargo update -p opentelemetry-otlp --precise 0.30.0 || true
cargo update -p opentelemetry-http --precise 0.30.0 || true
cargo update -p opentelemetry-proto --precise 0.30.0 || true
cargo update -p tracing-opentelemetry --precise 0.31.0 || true


# Relatório enxuto
if cargo tree -d | grep -E "opentelemetry(_sdk|-otlp|-http|-proto)?|tracing-opentelemetry" >/dev/null; then
echo "Duplicates:" && cargo tree -d | grep -E "opentelemetry(_sdk|-otlp|-http|-proto)?|tracing-opentelemetry"
else
echo "OK"
fi
