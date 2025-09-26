#!/usr/bin/env bash
set -euo pipefail

TOML="Cargo.toml"; LOCK="Cargo.lock"
cp -n "$TOML" "${TOML}.bak" 2>/dev/null || true
cp -n "$LOCK" "${LOCK}.bak" 2>/dev/null || true

# Remover exporters legados (se existirem)
cargo rm opentelemetry-prometheus 2>/dev/null || true
cargo rm opentelemetry-jaeger 2>/dev/null || true

# Base tracing (sem flags antigas)
cargo add tracing@0.1
cargo add tracing-subscriber@0.3

# Stack OTLP 0.29.x — um crate por comando quando há features
cargo add opentelemetry@0.29
cargo add opentelemetry_sdk@0.29 --features trace,metrics,rt-tokio
cargo add opentelemetry-otlp@0.29 --features http-proto,trace,metrics
cargo add tracing-opentelemetry@0.29

# Atualizar lock nas libs alvo
cargo update -p opentelemetry -p opentelemetry_sdk -p opentelemetry-otlp -p tracing-opentelemetry

# Relatório de duplicatas
cargo tree -d | grep -E "opentelemetry(_sdk|-otlp)?|tracing-opentelemetry" || true

echo -e "\n✅ Migração OTLP 0.29.x aplicada."
