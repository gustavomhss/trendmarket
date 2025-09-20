#!/usr/bin/env bash
set -euo pipefail
ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"; cd "$ROOT"
say(){ printf "\n[linklib] %s\n" "$*"; }


# 0) detectar nome do package e derivar o identificador do crate (hyphen -> underscore)
PKG_NAME=$(awk 'BEGIN{p=0} /^\[package\]/{p=1;next} /^\[/{if(p){exit} } p && /^name[[:space:]]*=/ {sub(/^[^=]*=[[:space:]]*"/,""); sub(/".*/,""); print; exit}' Cargo.toml)
if [ -z "${PKG_NAME:-}" ]; then echo "ERRO: não achei [package].name em Cargo.toml"; exit 10; fi
CRATE_IDENT="${PKG_NAME//-/_}"
say "package: $PKG_NAME -> crate ident: $CRATE_IDENT"


# 1) garantir [lib] ÚNICO com name + path corretos
say "regravando [lib] (único)"
python3 - <<'PY'
import io,re
p='Cargo.toml'
s=io.open(p,encoding='utf-8').read()
# remove TODOS os blocos [lib]
s=re.sub(r'\n\[lib\][^\[]*','\n',s,flags=re.S)
open(p,'w',encoding='utf-8').write(s)
PY
cat >> Cargo.toml <<EOF


[lib]
name = "${CRATE_IDENT}"
path = "src/lib.rs"
crate-type = ["rlib"]
EOF


# 2) sanidade das deps num-* em [dependencies] (não mexe se já estão certas)
if ! grep -q '^\[dependencies\]' Cargo.toml; then echo "[dependencies]" >> Cargo.toml; fi
for dep in 'num-bigint = "0.4"' 'num-integer = "0.1"' 'num-rational = "0.4"' 'num-traits = "0.2"'; do
k="${dep%% *}"; grep -qE "^[[:space:]]*$k[[:space:]]*=" Cargo.toml || printf '%s\n' "$dep" >> Cargo.toml
done


# 3) garantir lib.rs mínima e bridge (não força types/math na raiz)
mkdir -p src
if ! grep -q '^pub mod amm;' src/lib.rs 2>/dev/null; then
cat > src/lib.rs <<'RS'
/* lib (CRD-7-10) */
pub mod amm;
pub mod ce_core;
RS
fi
cat > src/ce_core.rs <<'RS'
/* Bridge ce_core — compatibiliza imports legados */
pub mod ce_core {
pub mod amm { pub use crate::amm::*; }
pub mod types { pub use crate::amm::types::*; }
}
RS


# 4) detectar onde está get_amount_out (swap/cpmm) para importar certo
MOD=swap; grep -R -n "get_amount_out" src/amm/cpmm 2>/dev/null && MOD=cpmm
say "get_amount_out em amm::$MOD"


# 5) recriar teste golden usando o identificador real do crate
mkdir -p tests
cat > tests/golden_cpmm.rs <<RS
//! Golden set CPMM (fee=0): |Δk/k| ≤ 1e-9
use ${CRATE_IDENT}::ce_core::amm::${MOD}::get_amount_out;
use ${CRATE_IDENT}::ce_core::amm::types::U256;
fn u256(s: &str) -> U256 { U256::from_dec_str(s).expect("u256 parse") }
fn e18() -> U256 { u256("1000000000000000000") }
fn check(name:&str, rx:U256, ry:U256, dx:U256){
let k0=rx*ry; let dy=get_amount_out(rx,ry,dx,0).expect("swap ok"); let k1=(rx+dx)*(ry-dy);
let delta=if k1>=k0{ k1-k0 }else{ k0-k1 }; let tol=k0/U256::from(1_000_000_000u64);
assert!(delta<=tol, "{}: |Δk|={} > tol={}", name, delta, tol);
}
#[test]
fn golden_cpmm_all(){
let w=e18();
// simetria
check("sym:small", u256("1000000")*w, u256("1000000")*w, u256("1000")*w);
check("sym:large", u256("5000000000")*w, u256("5000000000")*w, u256("1000000")*w);
// assimetria
check("asym:x>>y", u256("1000000000")*w, u256("1000000")*w, u256("1000")*w);
check("asym:y>>x", u256("1000000")*w, u256("1000000000")*w, u256("1000")*w);
// limites
check("lim:min_dx", u256("1000000")*w, u256("1000000")*w, U256::from(1u64));
check("lim:tiny_vs_big", u256("1000")*w, u256("1000000000")*w, u256("1")*w);
// seq add→swap→remove (invariância validada no swap)
let rx0=u256("2000000")*w; let ry0=u256("3000000")*w; let dx0=u256("500")*w; let s=U256::from(2u64);
check("seq:add→swap→remove", rx0*s, ry0*s, dx0);
}
RS


# 6) build isolado do golden
say "rodando o golden"
cargo test --test golden_cpmm -- --nocapture
say "PASS: golden_cpmm"
