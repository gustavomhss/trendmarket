#!/usr/bin/env bash
set -euo pipefail
cd "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"

say() { printf "\n[fix-crd-7-10] %s\n" "$*"; }
file_exists() { [ -f "$1" ]; }
dir_exists() { [ -d "$1" ]; }

say "1) Desligando binários que atrapalham (golden_runner)…"
# a) se existir bloco [[bin]] para golden_runner → remove com awk (BSD-safe)
if grep -q '^\[\[bin\]\]' Cargo.toml 2>/dev/null && grep -q 'name *= *"golden_runner"' Cargo.toml 2>/dev/null; then
  awk '
  BEGIN{inblk=0}
  /^\[\[bin\]\]/{buf=$0; inblk=1; next}
  inblk==1 {
    buf=buf"\n"$0;
    if ($0 ~ /^\[.*\]/) { # chegou no próximo bloco
      if (buf ~ /name *= *"golden_runner"/) { print "# (removed) golden_runner bin block" } else { print buf }
      inblk=0; buf=""; print $0; next
    } else { next }
  }
  {print}
  END{
    if (inblk==1) {
      if (buf ~ /name *= *"golden_runner"/) { print "# (removed) golden_runner bin block" } else { print buf }
    }
  }' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml
fi
# b) renomeia arquivo físico se ainda existir
if file_exists src/bin/golden_runner.rs; then
  mv src/bin/golden_runner.rs src/bin/golden_runner.rs.disabled
fi

say "2) Garantindo [dependencies] corretas (num-*)…"
# Injeta deps faltantes na seção [dependencies]
ensure_dep(){
  local crate="$1" ver="$2"
  if ! grep -qE "^[[:space:]]*${crate}[[:space:]]*=" Cargo.toml; then
    grep -q '^\[dependencies\]' Cargo.toml || printf '\n[dependencies]\n' >> Cargo.toml
    printf '%s = "%s"\n' "$crate" "$ver" >> Cargo.toml
  fi
}
# limpa entradas erradas em outras seções
sed -i '' -E '/^[[:space:]]*num-(bigint|integer|rational|traits)[[:space:]]*=.*/d' Cargo.toml || true
ensure_dep num-bigint 0.4
ensure_dep num-integer 0.1
ensure_dep num-rational 0.4
ensure_dep num-traits 0.2

say "3) Criando/normalizando src/lib.rs (expondo módulos reais)…"
mkdir -p src
{
  echo "/* auto-generated lib.rs (CRD-7-10) */"
  for m in amm types math util; do
    if [ -f "src/$m.rs" ] || [ -d "src/$m" ]; then echo "pub mod $m;"; fi
  done
} > src/lib.rs

say "4) Desligando testes que não são desta task (se houver)…"
if file_exists tests/rounding.rs; then
  mv tests/rounding.rs tests/rounding.rs.disabled
fi

say "5) Detectando caminhos corretos para imports (get_amount_out, U256)…"
# Detecta módulo do get_amount_out (swap|cpmm)
GET_MOD=""
GET_PATH="$(grep -R -n 'get_amount_out' src 2>/dev/null | head -n1 | cut -d: -f1 || true)"
case "$GET_PATH" in
  *"/amm/swap" | *"/amm/swap.rs") GET_MOD="amm::swap" ;;
  *"/amm/cpmm" | *"/amm/cpmm.rs") GET_MOD="amm::cpmm" ;;
  *) # fallback: tenta localizar mod pelo nome do arquivo
     if grep -REn '(^|[^a-zA-Z_])mod[[:space:]]+swap\b|pub[[:space:]]+mod[[:space:]]+swap\b' src >/dev/null 2>&1; then
       GET_MOD="amm::swap"
     fi
     if [ -z "$GET_MOD" ] && grep -REn '(^|[^a-zA-Z_])mod[[:space:]]+cpmm\b|pub[[:space:]]+mod[[:space:]]+cpmm\b' src >/dev/null 2>&1; then
       GET_MOD="amm::cpmm"
     fi
     ;;
esac
if [ -z "$GET_MOD" ]; then
  echo "ERRO: não encontrei get_amount_out em amm::swap/cpmm"; exit 12
fi
say "GET_MOD detectado: $GET_MOD"

say "OK."
