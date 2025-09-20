#!/usr/bin/env bash
set -euo pipefail
TREE=$(cargo tree --manifest-path Cargo.toml --color=never || true)


check_unique_mm() {
local crate="$1"
local vs; vs=$(printf "%s\n" "$TREE" | sed -n "s/.* \(${crate}\) v\([0-9]\+\.[0-9]\+\)\.[0-9]\+.*/\2/p" | sort -u)
local count; count=$(printf "%s\n" "$vs" | sed '/^$/d' | wc -l | tr -d ' ')
if [ "$count" -eq 0 ]; then echo "⚠️ ${crate}: não apareceu (ok se não é deps direta)"; return 0; fi
if [ "$count" -gt 1 ]; then echo "❌ ${crate}: múltiplas major.minor:"; printf " - %s\n" $vs; return 2; fi
echo "✅ ${crate}: major.minor única -> $(printf "%s" "$vs")"
}


fail=0
for c in opentelemetry opentelemetry_sdk opentelemetry-otlp opentelemetry-http opentelemetry-proto tracing-opentelemetry; do
check_unique_mm "$c" || fail=1
done


# Asserts desta task
if grep -q 'prost v0.14' <<<"$TREE"; then echo "❌ prost 0.14.* detectado (esperado 0.13.5)"; fail=1; fi


[ "$fail" -eq 0 ] && echo "✔️ Versões OTel uniformes e corretas." || { echo "Falhou checagem de versões"; exit 2; }
