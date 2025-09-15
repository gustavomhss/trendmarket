use proptest::prelude::*;
use credit_engine_core::amm::swap::get_amount_out; // se sua função morar em cpmm, troque swap->cpmm
use credit_engine_core::amm::types::{Wad, WAD, U256, Ppm};


#[inline]
fn to_wad(v: u128) -> Wad { v * WAD }
#[inline]
fn k(x: Wad, y: Wad) -> U256 { U256::from(x) * U256::from(y) }


proptest! {
#![proptest_config(ProptestConfig { cases: 10_000, .. ProptestConfig::default() })]


#[test]
fn invariants_hold(
rx_base in 1u128..=1_000_000_000u128,
ry_base in 1u128..=1_000_000_000u128,
dx_base in 1u128..=1_000_000u128,
fee_ppm in 0u32..=3000u32, // até 0.3%
) {
let (rx, ry, dx) = (to_wad(rx_base), to_wad(ry_base), to_wad(dx_base));
let k0 = k(rx, ry);


let dy: Wad = get_amount_out(rx, ry, dx, fee_ppm as Ppm).expect("swap ok");


// (P3) Sanidade: dy em (0, ry]
prop_assert!(dy > 0u128 && dy <= ry, "dy out of range: dy={}, ry={}", dy, ry);


let k1 = k(rx + dx, ry - dy);


if fee_ppm == 0 {
// (P1) Conservação de k (tolerância 1e-9)
let delta = if k1 >= k0 { k1 - k0 } else { k0 - k1 };
let tol = k0 / U256::from(1_000_000_000u64);
prop_assert!(delta <= tol,
"|Δk|={} > tol={} (k0={}, k1={}, rx={}, ry={}, dx={}, dy={})",
delta, tol, k0, k1, rx, ry, dx, dy);
} else {
// (P2) Com taxa: k' ≥ k
prop_assert!(k1 >= k0, "k' < k with fee: k0={}, k1={}, fee_ppm={}", k0, k1, fee_ppm);
}
}
}
