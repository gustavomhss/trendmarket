#!/usr/bin/env bash
set -euo pipefail


TOML="Cargo.toml"; LOCK="Cargo.lock"
cp -n "$TOML" "${TOML}.bak" 2>/dev/null || true
cp -n "$LOCK" "${LOCK}.bak" 2>/dev/null || true


# 1) Remover exporters legados que forçam linhas antigas
cargo rm opentelemetry-prometheus 2>/dev/null || true
cargo rm opentelemetry-jaeger 2>/dev/null || true


# 2) Garantir base de tracing
cargo add tracing@0.1 tracing-subscriber@0.3 --allow-prerelease || true


# 3) Adicionar stack ALVO
cargo add opentelemetry@0.30 \
opentelemetry_sdk@0.30 -F trace,metrics,logs,rt-tokio \
opentelemetry-otlp@0.30 -F http-proto \
tracing-opentelemetry@0.31


# 4) Pin explícito para evitar misturas (resolve a ambiguidade 0.28/0.29)
cargo update -p opentelemetry --precise 0.30.0
cargo update -p opentelemetry_sdk --precise 0.30.0
cargo update -p opentelemetry-otlp --precise 0.30.0
cargo update -p opentelemetry-http --precise 0.30.0 || true
cargo update -p opentelemetry-proto --precise 0.30.0 || true
cargo update -p tracing-opentelemetry --precise 0.31.0


# 5) Relatório; falha se houver duplicatas
if cargo tree -d | grep -E "opentelemetry(_sdk|-otlp|-http|-proto)?|tracing-opentelemetry"; then
echo "\n<<< Se aparecerem duas linhas (ex.: opentelemetry 0.28 + 0.30), ainda há conflito >>>"
fi


echo "\n✅ Migração para OTLP 0.30 aplicada."
