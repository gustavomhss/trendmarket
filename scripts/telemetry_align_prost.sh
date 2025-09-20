#!/usr/bin/env bash
set -euo pipefail

echo "🔎 Antes (prost no grafo):"
cargo tree --color=never | grep -E '(^| )prost(@| v|-)'

echo -e "\n🔧 Alinhando prost 0.14.1…"
cargo update -p prost --precise 0.14.1 || true
cargo update -p prost-types --precise 0.14.1 || true
cargo update -p prost-derive --precise 0.14.1 || true

echo -e "\n✅ Depois (prost no grafo):"
cargo tree --color=never | grep -E '(^| )prost(@| v|-)'

echo -e "\nDica: se ainda aparecer 0.13.x, rode:\n  cargo tree -i prost@0.13.* --color=never"
