#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"; cd "$ROOT"
say(){ printf "\n[CRD-7-10] %s\n" "$*"; }

say "1) Removendo [[bin]] golden_runner do Cargo.toml (se houver)"
python3 - <<'PY'
import re, sys, io
p = 'Cargo.toml'
try:
    s = io.open(p, encoding='utf-8').read()
except FileNotFoundError:
    sys.exit(0)
s = re.sub(r'\[\[bin\]\][^\[]*?name\s*=\s*"golden_runner"[^\[]*', '', s, flags=re.S)
io.open(p, 'w', encoding='utf-8').write(s)
PY


say "2) Garantindo [dependencies] (num-*) no local correto"
grep -q '^\[dependencies\]' Cargo.toml || printf '\n[dependencies]\n' >> Cargo.toml
for dep in 'num-bigint = "0.4"' 'num-integer = "0.1"' 'num-rational = "0.4"' 'num-traits = "0.2"'; do
  key="${dep%% *}"
  grep -qE "^[[:space:]]*$key[[:space:]]*=" Cargo.toml || printf '%s\n' "$dep" >> Cargo.toml
done


say "3) Gerando lib.rs e bridge ce_core (reexports)"
mkdir -p src
mods=""
for m in amm types math util; do
  if [ -f "src/$m.rs" ] || [ -d "src/$m" ]; then
    mods+=" $m"
  fi
done

# lib.rs
{
  echo "/* auto lib.rs (CRD-7-10) */"
  for m in $mods; do echo "pub mod $m;"; done
  echo "pub mod ce_core;"
} > src/lib.rs

# ce_core.rs (bridge de reexports)
{
  echo "/* ce_core bridge: mapeia para crate::* */"
  echo "pub mod ce_core {"

  if [ -d src/amm ] || [ -f src/amm.rs ]; then
    echo "  pub mod amm {"
    for f in src/amm/*.rs; do
      [ -e "$f" ] || continue
      b="$(basename "$f" .rs)"
      echo "    pub use crate::amm::$b;"
    done
    for d in src/amm/*; do
      [ -d "$d" ] || continue
      [ -f "$d/mod.rs" ] || continue
      b="$(basename "$d")"
      echo "    pub use crate::amm::$b;"
    done
    echo "  }"
  fi

  if [ -f src/amm/types.rs ] || [ -d src/amm/types ] || [ -f src/types.rs ] || [ -d src/types ]; then
    echo "  pub mod types {"
    if [ -f src/amm/types.rs ] || [ -d src/amm/types ]; then
      echo "    pub use crate::amm::types::*;"
    else
      echo "    pub use crate::types::*;"
    fi
    echo "  }"
  fi

  if [ -f src/amm/math.rs ] || [ -d src/amm/math ] || [ -f src/math.rs ] || [ -d src/math ]; then
    echo "  pub mod math {"
    if [ -f src/amm/math.rs ] || [ -d src/amm/math ]; then
      echo "    pub use crate::amm::math::*;"
    else
      echo "    pub use crate::math::*;"
    fi
    echo "  }"
  fi

  echo "}"
} > src/ce_core.rs


say "4) Detectando paths de get_amount_out e U256"
GET_MOD=""
if [ -f src/amm/swap.rs ] || [ -d src/amm/swap ]; then
  GET_MOD="amm::swap"
fi
if [ -z "$GET_MOD" ] && { [ -f src/amm/cpmm.rs ] || [ -d src/amm/cpmm ]; }; then
  GET_MOD="amm::cpmm"
fi
if [ -z "$GET_MOD" ]; then
  gp="$(grep -R -n 'get_amount_out' src 2>/dev/null | head -n1 | cut -d: -f1 || true)"
  case "$gp" in
    *"/amm/swap"*) GET_MOD="amm::swap" ;;
    *"/amm/swap.rs"*) GET_MOD="amm::swap" ;;
    *"/amm/cpmm"*) GET_MOD="amm::cpmm" ;;
    *"/amm/cpmm.rs"*) GET_MOD="amm::cpmm" ;;
  esac
fi
if [ -z "$GET_MOD" ]; then
  echo "ERRO: não encontrei get_amount_out (amm::swap/cpmm)."
  exit 2
fi

U256_MOD=""
if [ -f src/amm/types.rs ] || [ -d src/amm/types ]; then
  U256_MOD="amm::types"
elif [ -f src/types.rs ] || [ -d src/types ]; then
  U256_MOD="types"
elif [ -f src/math.rs ] || [ -d src/math ]; then
  U256_MOD="math"
fi
if [ -z "$U256_MOD" ]; then
  echo "ERRO: não encontrei U256 (amm::types/types/math)."
  exit 3
fi

# Descobre o nome do crate (package.name -> crate_name com _)
CRATE_NAME="$(python3 - <<'PY'
import re, io
try:
    s = io.open("Cargo.toml", encoding="utf-8").read()
except:
    print("crate")
    raise SystemExit
m = re.search(r'^\s*\[package\][\s\S]*?^\s*name\s*=\s*"([^"]+)"', s, re.M)
name = m.group(1) if m else "crate"
print(name.replace("-", "_"))
PY
)"


say "5) Gerando teste golden completo (4 famílias de casos, fee=0, |Δk/k| ≤ 1e-9)"
mkdir -p tests
cat > tests/golden_cpmm.rs <<RS
//! Golden set CPMM (fee=0): |Δk/k| ≤ 1e-9 em todos os casos
use ${CRATE_NAME}::${GET_MOD}::get_amount_out;
use ${CRATE_NAME}::${U256_MOD}::U256;

fn u256(s: &str) -> U256 { U256::from_dec_str(s).expect("u256 parse") }
fn e18() -> U256 { u256("1000000000000000000") }

fn check_invariance(name: &str, rx: U256, ry: U256, dx: U256) {
    let k0 = rx * ry;
    let dy = get_amount_out(rx, ry, dx, 0).expect("swap ok");
    let k1 = (rx + dx) * (ry - dy);
    let delta = if k1 >= k0 { k1 - k0 } else { k0 - k1 };
    let tol = k0 / U256::from(1_000_000_000u64);
    assert!(delta <= tol, "{}: |Δk|={} > tol={} (rx={}, ry={}, dx={}, dy={})", name, delta, tol, rx, ry, dx, dy);
}

#[test]
fn golden_cpmm_all() {
    let w = e18();
    // 1) Simetria
    check_invariance("sym:balanced_small", u256("1000000")*w, u256("1000000")*w, u256("1000")*w);
    check_invariance("sym:balanced_large", u256("5000000000")*w, u256("5000000000")*w, u256("1000000")*w);

    // 2) Assimetria
    check_invariance("asym:x>>y", u256("1000000000")*w, u256("1000000")*w, u256("1000")*w);
    check_invariance("asym:y>>x", u256("1000000")*w, u256("1000000000")*w, u256("1000")*w);

    // 3) Limites (dx mínimo; reservas altas/baixas)
    check_invariance("lim:min_dx", u256("1000000")*w, u256("1000000")*w, U256::from(1u64));
    check_invariance("lim:tiny_reserve_vs_big", u256("1000")*w, u256("1000000000")*w, u256("1")*w);

    // 4) Sequência add→swap→remove (simulada com escala proporcional)
    // Escala reservas (add), faz swap, reescala de volta (remove). Invariância é checada no swap.
    let rx0 = u256("2000000")*w; let ry0 = u256("3000000")*w; let dx0 = u256("500")*w;
    let s = U256::from(2u64); // simula add de liquidez (x2)
    let rx1 = rx0 * s; let ry1 = ry0 * s; // add
    check_invariance("seq:add→swap→remove", rx1, ry1, dx0);
    // remove (escala de volta) — não precisa checar k aqui, pois add/remove alteram k por definição; foco é swap
}
RS


say "6) Rodando somente este teste"
cargo test --test golden_cpmm -- --nocapture
say "OK — golden_cpmm PASS"


say "7) Commit local + artefatos (sem push)"
mkdir -p scripts goldens
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git add src/lib.rs src/ce_core.rs tests/golden_cpmm.rs scripts/fix_crd_7_10_final.sh || true
  git commit -m "test(crd-7-10): golden CPMM (fee=0) |Δk/k|≤1e-9 + build isolation + ce_core bridge" || true
  git rev-parse --short HEAD > .commit_sha.txt || true
  COMMIT_SHA="$(cat .commit_sha.txt 2>/dev/null || echo "<NO-COMMIT>")"
  git format-patch -1 HEAD --stdout > crd-7-10.patch || true
  tar -czf crd-7-10-artifacts.tgz \
    src/lib.rs src/ce_core.rs \
    tests/golden_cpmm.rs \
    scripts/run_golden.sh 2>/dev/null || true
  say "Artefatos: crd-7-10.patch, crd-7-10-artifacts.tgz (Commit: ${COMMIT_SHA})"
fi
