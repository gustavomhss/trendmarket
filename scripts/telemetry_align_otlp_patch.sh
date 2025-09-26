#!/usr/bin/env bash
set -euo pipefail

echo "🔧 Alinhando todo o stack OTel para 0.30.1 e prost 0.14.1…"

# Alinhar TODOS os crates OTel para o MESMO patch (0.30.1)
for C in \
  opentelemetry \
  opentelemetry_sdk \
  opentelemetry-otlp \
  opentelemetry-http \
  opentelemetry-proto
do
  cargo update -p "$C" --precise 0.30.1 || true
done

# Garantir prost no patch certo (evita 0.13.* no grafo)
for C in prost prost-types prost-derive; do
  cargo update -p "$C" --precise 0.14.1 || true
done

echo -e "\n🔎 Verificando prost no grafo:"
cargo tree --color=never | grep -E '(^| )prost(@| v|-)'

echo -e "\n🔎 Verificando versões dos OTel crates:"
cargo tree --color=never | grep -E 'opentelemetry(-sdk|-otlp|-http|-proto)? v0\.30\.[0-9]+|tracing-opentelemetry v0\.31\.[0-9]+'

echo -e "\n✅ Alinhamento feito. Tente compilar: cargo build -q"
