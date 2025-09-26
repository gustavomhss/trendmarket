#!/usr/bin/env bash
set -euo pipefail


echo "🔧 Removendo deps antigas do OpenTelemetry…"
for DEP in \
opentelemetry \
opentelemetry_sdk \
opentelemetry-otlp \
opentelemetry-http \
opentelemetry-proto \
tracing-opentelemetry \
opentelemetry-prometheus \
opentelemetry-jaeger
do
cargo rm -q "$DEP" 2>/dev/null || true
done


# utilitários
cargo add anyhow@1 --quiet || true


echo "➕ Adicionando deps alvo (OTLP 0.30 + tracing)…"
# Adicione uma por vez para aplicar features ao crate correto
cargo add tracing@0.1 --quiet
cargo add tracing-subscriber@0.3 --features env-filter,fmt,registry,tracing-log --quiet


# Core OTel 0.30 (sem pin de patch; permite 0.30.x)
cargo add opentelemetry@0.30 --quiet
cargo add opentelemetry_sdk@0.30 --features trace,metrics,rt-tokio --quiet


# Exportadores OTLP HTTP (trace+metrics)
cargo add opentelemetry-otlp@0.30 --features trace,metrics,http-proto --quiet


# Bridge com tracing 0.31
cargo add tracing-opentelemetry@0.31 --quiet


# Atualiza lock do workspace inteiro
cargo update -w


echo "✅ Dependências definidas; rode o assert de versões."
