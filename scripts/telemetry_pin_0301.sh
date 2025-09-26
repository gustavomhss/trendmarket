#!/usr/bin/env bash
set -euo pipefail

TOML="Cargo.toml"; LOCK="Cargo.lock"
cp -n "$TOML" "${TOML}.bak" 2>/dev/null || true
cp -n "$LOCK" "${LOCK}.bak" 2>/dev/null || true

echo "🔧 Limpando deps OTel…"
for DEP in \
  opentelemetry \
  opentelemetry_sdk \
  opentelemetry-otlp \
  opentelemetry-http \
  opentelemetry-proto \
  tracing-opentelemetry
do
  cargo rm -q "$DEP" 2>/dev/null || true
done

echo "➕ Repondo deps com patches exatos (0.30.1 / 0.31.0)…"
cargo add tracing@=0.1.40 -q || true
cargo add tracing-subscriber@=0.3.18 --features env-filter,fmt,registry,tracing-log -q || true

# OTel 0.30.1 exato em TODOS os crates do stack
cargo add opentelemetry@=0.30.1 -q
cargo add opentelemetry_sdk@=0.30.1 --features trace,metrics,rt-tokio -q
cargo add opentelemetry-otlp@=0.30.1 --features trace,metrics,http-proto -q
cargo add opentelemetry-http@=0.30.1 -q
cargo add opentelemetry-proto@=0.30.1 -q

# Bridge com tracing
cargo add tracing-opentelemetry@=0.31.0 -q

# Garantir prost unificado (0.14.1); adicionar como dep direta ajuda a resolver
cargo add prost@=0.14.1 -q || true
cargo add prost-types@=0.14.1 -q || true

echo "🧹 Regenerando lock do zero para evitar resíduos…"
rm -f Cargo.lock
cargo generate-lockfile

echo "🔎 Checando prost 0.13 no grafo…"
if cargo tree --color=never -i 'prost@0.13.*' | grep -q 'prost v0\.13'; then
  echo "❌ Ainda existe prost 0.13.* no grafo. Saída:"
  cargo tree --color=never -i 'prost@0.13.*' || true
  exit 2
fi

echo "✅ prost unificado em 0.14.x. Tentando build…"
cargo build -q && echo "✅ Build OK."
