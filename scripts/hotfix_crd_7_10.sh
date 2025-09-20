#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"; cd "$ROOT"
say(){ printf "\n[CRD-7-10] %s\n" "$*"; }

# 1) Bridge ce_core — compatibiliza imports legados
mkdir -p src

HAS_AMM_TYPES=0
if [ -f src/amm/types.rs ] || [ -d src/amm/types ]; then
  HAS_AMM_TYPES=1
fi

# ce_core.rs
cat > src/ce_core.rs <<'RS'
/* Bridge ce_core — compatibiliza imports legados */
pub mod ce_core {
    pub mod amm { pub use crate::amm::*; }
    pub mod types {
RS
if [ "$HAS_AMM_TYPES" -eq 1 ]; then
  echo '        pub use crate::amm::types::*;' >> src/ce_core.rs
else
  echo '        pub use crate::types::*;' >> src/ce_core.rs
fi
cat >> src/ce_core.rs <<'RS'
    }
}
RS

# 2) Detectando módulo com get_amount_out (swap/cpmm)
say "2) Detectando módulo com get_amount_out (swap/cpmm)"
GET_MOD=""
if [ -f src/amm/swap.rs ] || [ -d src/amm/swap ]; then
  GET_MOD="amm::swap"
elif [ -f src/amm/cpmm.rs ] || [ -d src/amm/cpmm ]; then
  GET_MOD="amm::cpmm"
else
  gp="$(grep -R -n 'get_amount_out' src 2>/dev/null | head -n1 | cut -d: -f1 || true)"
  case "$gp" in
    *"/amm/swap"*) GET_MOD="amm::swap" ;;
    *"/amm/swap.rs"*) GET_MOD="amm::swap" ;;
    *"/amm/cpmm"*) GET_MOD="amm::cpmm" ;;
    *"/amm/cpmm.rs"*) GET_MOD="amm::cpmm" ;;
  esac
fi
if [ -z "$GET_MOD" ]; then
  echo "ERRO: não encontrei get_amount_out em amm::swap/cpmm"; exit 12
fi

# 3) Nome do crate (transforma '-' em '_')
say "3) Detectando nome do crate"
CRATE_NAME="$(python3 - <<'PY'
import re, io
try:
    s = io.open("Cargo.toml", encoding="utf-8").read()
except FileNotFoundError:
    print("crate"); raise SystemExit
m = re.search(r'^\s*\[package\][\s\S]*?^\s*name\s*=\s*"([^"]+)"', s, re.M)
name = m.group(1) if m else "crate"
print(name.replace("-", "_"))
PY
)"

# Caminho da U256 conforme presença de amm::types
if [ "$HAS_AMM_TYPES" -eq 1 ]; then
  U256_PATH="${CRATE_NAME}::ce_core::amm::types::U256"
else
  U256_PATH="${CRATE_NAME}::ce_core::types::U256"
fi

# 4) Teste golden
say "4) Criando teste golden (fee=0, |Δk/k|≤1e-9)"
mkdir -p tests
cat > tests/golden_cpmm.rs <<RS
//! Golden set CPMM (fee=0): |Δk/k| ≤ 1e-9
use ${CRATE_NAME}::ce_core::${GET_MOD}::get_amount_out;
use ${U256_PATH};

fn u256(s: &str) -> U256 { U256::from_dec_str(s).expect("u256 parse") }
fn e18() -> U256 { u256("1000000000000000000") }

fn check(name:&str, rx:U256, ry:U256, dx:U256){
    let k0 = rx*ry;
    let dy = get_amount_out(rx,ry,dx,0).expect("swap ok");
    let k1 = (rx+dx)*(ry-dy);
    let delta = if k1>=k0 { k1-k0 } else { k0-k1 };
    let tol = k0 / U256::from(1_000_000_000u64);
    assert!(delta <= tol, "{}: |Δk|={} > tol={}", name, delta, tol);
}

#[test]
fn golden_cpmm_all(){
    let w = e18();
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

# 5) Build isolado do teste
say "5) Build do golden — isolado"
cargo test --test golden_cpmm -- --nocapture
say "OK: golden_cpmm PASS (fee=0)"
