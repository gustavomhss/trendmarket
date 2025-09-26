#!/usr/bin/env bash
set -euo pipefail


TOML="Cargo.toml"; LOCK="Cargo.lock"
cp -n "$TOML" "${TOML}.bak" 2>/dev/null || true
cp -n "$LOCK" "${LOCK}.bak" 2>/dev/null || true


echo "[reset] removendo deps OTel/legadas…"
for DEP in \
opentelemetry \
opentelemetry_sdk \
opentelemetry-otlp \
opentelemetry-http \
opentelemetry-proto \
tracing-opentelemetry \
opentelemetry-prometheus \
opentelemetry-jaeger \
prost \
prost-types
do cargo rm -q "$DEP" 2>/dev/null || true; done


# Base
cargo add anyhow@1 -q || true
cargo add tracing@0.1.40 -q || true
cargo add tracing-subscriber@0.3.18 --features env-filter,fmt,registry,tracing-log -q || true
cargo add tokio@1 -F macros,rt-multi-thread,time -q || true


# OTel 0.30.0 (patch EXATO)
cargo add opentelemetry@=0.30.0 -q
cargo add opentelemetry_sdk@=0.30.0 --features trace,metrics,rt-tokio -q
cargo add opentelemetry-otlp@=0.30.0 --features trace,metrics,http-proto -q
cargo add opentelemetry-http@=0.30.0 -q
cargo add opentelemetry-proto@=0.30.0 -q


# Bridge tracing
cargo add tracing-opentelemetry@=0.31.0 -q


# Lock limpo + unificar prost 0.13.5
rm -f Cargo.lock
cargo generate-lockfile


# Atualiza prost* somente se existirem no grafo
if cargo tree --color=never | grep -qE '( |^)prost v'; then
cargo update -p prost --precise 0.13.5 || true
fi
if cargo tree --color=never | grep -qE '( |^)prost-types v'; then
cargo update -p prost-types --precise 0.13.5 || true
fi
if cargo tree --color=never | grep -qE '( |^)prost-derive v'; then
cargo update -p prost-derive --precise 0.13.5 || true
fi


# Verificação sintética (sem usar "-i" com semver inválido)
if cargo tree --color=never | grep -q 'prost v0\.14\.'; then
echo "[reset] ❌ prost 0.14.* detectado. Quem puxa:"; cargo tree --color=never | sed -n '1,200p' | grep -E 'prost v0\.14\.|opentelemetry-proto|opentelemetry-otlp|tonic' || true; exit 2
fi


echo "[reset] ✅ stack OTel 0.30.0 + tracing 0.31.0 + prost 0.13.5 pronto."
