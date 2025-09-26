#!/usr/bin/env bash
set -euo pipefail
say(){ printf "\n[fix-crd-7-10 v2] %s\n" "$*"; }
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"; cd "$ROOT"

# 0) localizar o arquivo do golden_runner (ativo ou .disabled)
GR=""; for p in src/bin/golden_runner.rs src/bin/golden_runner.rs.disabled; do [ -f "$p" ] && GR="$p" && break; done
[ -z "$GR" ] && { echo "ERRO: não achei src/bin/golden_runner.rs(.disabled)"; exit 10; }

say "1) Tentando extrair bloco inline: mod ce_core { ... }"
mkdir -p tmp src
python3 - "$GR" <<'PY'
import sys
path=sys.argv[1]
s=open(path,'r',encoding='utf-8').read()
needle='mod ce_core'
i=s.find(needle)
if i<0:
  print('NO_INLINE')
  sys.exit(0)
# encontra a primeira chaveta "{" após o token
j=s.find('{',i)
if j<0:
  print('NO_BRACE')
  sys.exit(2)
# varre balanceando chaves
k=j+1; depth=1
while k<len(s):
  c=s[k]
  if c=='{': depth+=1
  elif c=='}':
    depth-=1
    if depth==0:
      end=k
      break
  k+=1
else:
  print('NO_CLOSE')
  sys.exit(3)
inner=s[j+1:end]
open('tmp/ce_core.inner.rs','w',encoding='utf-8').write(inner)
print('OK')
PY

if [ -f tmp/ce_core.inner.rs ]; then
  say "2) Criando src/ce_core.rs com submódulos públicos"
  printf '/* extracted from %s */\n' "$GR" > src/ce_core.rs
  printf 'pub mod ce_core {\n' >> src/ce_core.rs
  # publica submódulos (mod X; -> pub mod X;  e  mod X { -> pub mod X {)
  sed -e 's/^\([[:space:]]*\)mod\s\+/\1pub mod /' \
      -e 's/\bmod\s\+/pub mod /' \
      tmp/ce_core.inner.rs >> src/ce_core.rs
  printf '\n}\n' >> src/ce_core.rs

  say "3) Garantindo lib.rs expondo ce_core"
  if ! grep -q '^pub mod ce_core;' src/lib.rs 2>/dev/null; then
    { echo '/* ensured by fix v2 */'; cat src/lib.rs 2>/dev/null; echo 'pub mod ce_core;'; } > src/lib.rs.new
    mv src/lib.rs.new src/lib.rs
  fi

  say "4) Detectando caminhos (swap/cpmm, U256) dentro do ce_core extraído"
  GET_MOD=""; if grep -qE '\bmod[[:space:]]+swap\b|\bpub[[:space:]]+mod[[:space:]]+swap\b' src/ce_core.rs; then GET_MOD="ce_core::amm::swap"; fi
  if [ -z "$GET_MOD" ] && grep -qE '\bmod[[:space:]]+cpmm\b|\bpub[[:space:]]+mod[[:space:]]+cpmm\b' src/ce_core.rs; then GET_MOD="ce_core::amm::cpmm"; fi
  [ -z "$GET_MOD" ] && { echo "ERRO: não achei swap/cpmm dentro de ce_core extraído"; exit 21; }

  U256_MOD=""; for c in 'ce_core::amm::types' 'ce_core::types' 'ce_core::math'; do
    base=$(echo "$c"|sed 's/.*:://')
    if grep -qE "\b(mod|pub mod) ${base}\b|\btype U256\b|\bstruct U256\b" src/ce_core.rs; then U256_MOD="$c"; break; fi
  done
  [ -z "$U256_MOD" ] && { echo "ERRO: não achei módulo com U256 dentro de ce_core"; exit 22; }

  say "5) (Re)gerando teste com imports corretos"
  mkdir -p tests
  cat > tests/golden_cpmm.rs <<RS
//! Golden set mínimo para CPMM (sem taxa): |Δk/k| ≤ 1e-9
use credit_engine_core::${GET_MOD}::get_amount_out;
use credit_engine_core::${U256_MOD}::U256;
fn u256(s: &str) -> U256 { U256::from_dec_str(s).expect("u256 parse") }
#[test]
fn golden_cpmm_invariance_basic() {
    let e18 = u256("1000000000000000000");
    let rx = u256("1000000")*e18; let ry = u256("1000000")*e18; let dx = u256("1000")*e18;
    let k0 = rx*ry; let dy = get_amount_out(rx, ry, dx, 0).expect("swap ok"); let k1 = (rx+dx)*(ry-dy);
    let delta = if k1>=k0 { k1-k0 } else { k0-k1 }; let tol = k0 / U256::from(1_000_000_000u64);
    assert!(delta <= tol, "|Δk|={} > tol={}", delta, tol);
}
RS

  say "6) Rodando somente este teste"
  cargo test --test golden_cpmm -- --nocapture
  say "OK — golden_cpmm rodou com ce_core extraído"
else
  say "Nenhum bloco inline de ce_core dentro do golden_runner; pular extração."
  exit 1
fi
