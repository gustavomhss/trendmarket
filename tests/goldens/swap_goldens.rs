// PATH: tests/goldens/swap_goldens.rs
//! Goldens para CPMM comparando core (U256+WAD) vs referência BigRational.


use std::str::FromStr;
use num_bigint::BigUint;


// Ajuste os caminhos conforme o crate do core
use credit_engine_core::amm::swap::{get_amount_out, get_amount_in};
use credit_engine_core::amm::pricing::{spot_price_y_per_x, exec_price_y_per_x};
use credit_engine_core::amm::ref_bigdecimal::{RefPool, Rounding, wad_round_big};


fn u256_to_big(u: credit_engine_core::U256) -> BigUint {
BigUint::from_str(&u.to_string()).expect("U256->BigUint parse")
}


#[test]
fn golden_amount_out_matches_core_floor() {
let fee_bps = 30; // 0.30%
let x = credit_engine_core::U256::from_dec_str("1000000000000000000000").unwrap(); // 1_000 WAD
let y = credit_engine_core::U256::from_dec_str("2000000000000000000000").unwrap(); // 2_000 WAD
let dx = credit_engine_core::U256::from_dec_str("500000000000000000").unwrap(); // 0.5 WAD


// Core
let dy_core = get_amount_out(x, y, dx, fee_bps);


// Ref
let pool = RefPool::new(1_000 * 10u128.pow(18), 2_000 * 10u128.pow(18), fee_bps);
let dy_ref_exact = pool.amount_out_exact(5 * 10u128.pow(17));
let dy_ref_wad = wad_round_big(&dy_ref_exact, Rounding::Floor);


assert_eq!(u256_to_big(dy_core), dy_ref_wad, "amount_out core deve ser floor(exato)");


// |Δk/k| usando dy_core observado
let dy_core_u128: u128 = dy_core.to_string().parse().unwrap();
let dk_over_k = pool.delta_k_over_k_after(5 * 10u128.pow(17), dy_core_u128);


// Bound: |Δk/k| ≤ 2e-18
let lhs_num = dk_over_k.numer().abs().to_biguint().unwrap();
let lhs_den = dk_over_k.denom().clone();
let bound_num = BigUint::from(2u32);
let bound_den = BigUint::from(10u128.pow(18));
assert!(lhs_num * &bound_den <= bound_num * lhs_den, "|Δk/k| excedeu o bound");
}


#[test]
fn golden_amount_in_matches_core_ceil() {
let fee_bps = 50; // 0.50%
let x = credit_engine_core::U256::from_dec_str("3000000000000000000000").unwrap(); // 3_000 WAD
let y = credit_engine_core::U256::from_dec_str("1000000000000000000000").unwrap(); // 1_000 WAD
let want_dy = credit_engine_core::U256::from_dec_str("1000000000000000000").unwrap(); // 1.0 WAD


// Core
let dx_core = get_amount_in(x, y, want_dy, fee_bps);


// Ref
let pool = RefPool::new(3_000 * 10u128.pow(18), 1_000 * 10u128.pow(18), fee_bps);
let dx_ref_exact = pool.amount_in_exact(1_000 * 10u128.pow(18));
let dx_ref_wad = wad_round_big(&dx_ref_exact, Rounding::Ceil);


assert_eq!(u256_to_big(dx_core), dx_ref_wad, "amount_in core deve ser ceil(exato)");
}


#[test]
fn golden_pricing_spot_exec_nearest_even() {
let fee_bps = 30;
let pool = RefPool::new(1_000 * 10u128.pow(18), 2_000 * 10u128.pow(18), fee_bps);
let dx = 1 * 10u128.pow(17); // 0.1 WAD


// Ref
let p_spot = pool.spot_price_y_per_x();
let p_exec = pool.exec_price_y_per_x(dx);
let spot_w = wad_round_big(&p_spot, Rounding::NearestEven);
let exec_w = wad_round_big(&p_exec, Rounding::NearestEven);


// Core
let x = credit_engine_core::U256::from_dec_str("1000000000000000000000").unwrap();
let y = credit_engine_core::U256::from_dec_str("2000000000000000000000").unwrap();
let dx_u = credit_engine_core::U256::from_dec_str("100000000000000000").unwrap();
let spot_core = spot_price_y_per_x(x, y);
let exec_core = exec_price_y_per_x(x, y, dx_u, fee_bps);


assert_eq!(u256_to_big(spot_core), spot_w);
assert_eq!(u256_to_big(exec_core), exec_w);
}
