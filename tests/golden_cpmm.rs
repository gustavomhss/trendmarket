//! Golden set CPMM (fee=0): |Δk/k| ≤ 1e-9 usando Wad nos inputs e U256 só para k
use credit_engine_core::amm::swap::get_amount_out; // se sua função estiver em cpmm, troque swap->cpmm
use credit_engine_core::amm::types::{Wad, WAD, U256};


#[inline] fn w(n: &str) -> Wad { n.parse::<u128>().expect("u128") * WAD }
#[inline] fn k(x: Wad, y: Wad) -> U256 { U256::from(x) * U256::from(y) }


fn check(name: &str, rx: Wad, ry: Wad, dx: Wad) {
let k0 = k(rx, ry);
let dy: Wad = get_amount_out(rx, ry, dx, 0u32).expect("swap ok");
let k1 = k(rx + dx, ry - dy);
let delta = if k1 >= k0 { k1 - k0 } else { k0 - k1 };
let tol = k0 / U256::from(1_000_000_000u64);
assert!(delta <= tol, "{}: |Δk|={} > tol={} (rx={}, ry={}, dx={}, dy={})", name, delta, tol, rx, ry, dx, dy);
}


#[test]
fn golden_cpmm_all() {
// 1e18 escala (WAD)
let rx = w("1000000");
let ry = w("1000000");
let dx = w("1000");
check("sym:small", rx, ry, dx);


check("sym:large", w("5000000000"), w("5000000000"), w("1000000"));


// assimetria
check("asym:x>>y", w("1000000000"), w("1000000"), w("1000"));
check("asym:y>>x", w("1000000"), w("1000000000"), w("1000"));


// limites
check("lim:min_dx", w("1000000"), w("1000000"), 1u128); // 1 wei
check("lim:tiny_vs_big", w("1000"), w("1000000000"), w("1"));


// sequência add→swap→remove (invariância validada no swap)
let s: Wad = 2u128; // fator de escala (add)
check("seq:add→swap→remove", w("2000000")*s, w("3000000")*s, w("500"));
}
