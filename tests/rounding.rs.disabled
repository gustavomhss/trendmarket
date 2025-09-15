//! Testes de direção de arredondamento (política única ADR-0001/0002)

use ce_core::amm::liquidity::{initial_mint, remove_liquidity};
use ce_core::amm::swap::{get_amount_in, get_amount_out};
use ce_core::amm::guardrails::{div_nearest_even_u256, div_nearest_even_u256_to_u128};
use ce_core::amm::types::{U256, Ppm, WAD, MIN_RESERVE};
use ce_core::amm::errors::AmmError;

const FEE0: Ppm = 0;
const FEE3: Ppm = 3000; // 0,30%

#[test]
fn r1_amount_out_is_floor_of_continuous_value() {
    // reservas bem acima do mínimo
    let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD);
    let out = get_amount_out(x, y, dx, FEE0).unwrap();
    // y* = (x*y)/x' com nearest-even (U256) → u128
    let x1 = x + dx; // sem taxa
    let k = U256::from(x) * U256::from(y);
    let y_star = div_nearest_even_u256_to_u128(k, U256::from(x1)).unwrap();
    // out deve ser exatamente floor(y - y*)
    assert_eq!(out, y - y_star);
    // e y' respeita o mínimo
    assert!(y - out >= MIN_RESERVE);
}

#[test]
fn r2_amount_in_is_ceil_minimality() {
    let (x, y, dy) = (1_000_000u128*WAD, 1_000_000u128*WAD, 9_870u128*WAD);
    let dx = get_amount_in(x, y, dy, FEE3).unwrap();
    // com dx-1 não deve alcançar dy
    if dx > 0 {
        let out_prev = get_amount_out(x, y, dx - 1, FEE3).unwrap_or(0);
        assert!(out_prev < dy);
    }
    let out = get_amount_out(x, y, dx, FEE3).unwrap();
    assert!(out >= dy);
}

#[test]
fn r3_fee_is_ceil_no_undercollection() {
    // fee mínima diferente de zero deve ser cobrada integralmente
    let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 1u128);
    let fee_ppm: Ppm = 1; // 0.0001%
    let err = get_amount_out(x, y, dx, fee_ppm).unwrap_err();
    assert_eq!(err, AmmError::InputTooSmall); // dx_net = 0
}

#[test]
fn r4_mint_is_floor_of_sqrt_xy() {
    let (x, y) = (2_500_000u128*WAD, 2_500_000u128*WAD);
    let s = initial_mint(x, y).unwrap();
    let k = U256::from(x) * U256::from(y);
    let s_plus = U256::from(s + 1);
    // (s+1)^2 deve ultrapassar k ⇒ s é floor(sqrt(k))
    assert!(s_plus * s_plus > k);
}

#[test]
fn r5_burn_amounts_are_floor_of_proportion() {
    let (x, y, s) = (1_000_000u128*WAD, 3_000_000u128*WAD, 1_000_000u128*WAD);
    let burn = 123_456u128*WAD; // ~12.3456%
    let (xo, yo) = remove_liquidity(x, y, burn, s).unwrap();
    // proporções teóricas (u256) e floors
    let xo_theo = ((U256::from(x) * U256::from(burn)) / U256::from(s)).as_u128();
    let yo_theo = ((U256::from(y) * U256::from(burn)) / U256::from(s)).as_u128();
    assert_eq!(xo, xo_theo);
    assert_eq!(yo, yo_theo);
    // reservas remanescentes ainda >= mínimo
    assert!(x - xo >= MIN_RESERVE && y - yo >= MIN_RESERVE);
}

#[test]
fn r6_intermediate_rounding_is_nearest_even_on_tie() {
    // constrói uma divisão U256 com empate exato: 5/2 e 3/2
    let two = U256::from(2u8);
    let three = U256::from(3u8);
    let five = U256::from(5u8);
    // 5/2 = 2.5 → empata; 2 é par → fica 2
    let q = div_nearest_even_u256(five, two).unwrap();
    assert_eq!(q, U256::from(2u8));
    // 3/2 = 1.5 → empata; 1 é ímpar → sobe para 2
    let q = div_nearest_even_u256(three, two).unwrap();
    assert_eq!(q, U256::from(2u8));
}
