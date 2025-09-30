use credit_engine_core::amm::{
    guardrails::{div_nearest_even_u256_to_u128, u256_to_u128_checked},
    liquidity, pricing, swap,
    types::{Ppm, Wad, PPM_SCALE, U256, WAD},
};

fn ceil_div_u256(n: U256, d: U256) -> U256 {
    (n + (d - U256::from(1u8))) / d
}

#[test]
fn readme_ex1_quote_in_numbers_match() {
    let x: Wad = 50_000 * WAD;
    let y: Wad = 80_000 * WAD;
    let dx: Wad = 1_234 * WAD;
    let fee_ppm: Ppm = 3_000;

    let n_fee = U256::from(dx) * U256::from(fee_ppm as u64);
    let d_fee = U256::from(PPM_SCALE as u64);
    let dx_fee = u256_to_u128_checked(ceil_div_u256(n_fee, d_fee)).unwrap();
    assert_eq!(dx_fee, 3_702_000_000_000_000_000u128);

    let dx_net = dx - dx_fee;
    assert_eq!(dx_net, 1_230_298_000_000_000_000_000u128);

    let x1 = x + dx_net;
    let k = U256::from(x) * U256::from(y);
    let y_star = div_nearest_even_u256_to_u128(k, U256::from(x1)).unwrap();
    assert_eq!(y_star, 78_078_796_262_321_175_644_928u128);

    let out = swap::get_amount_out(x, y, dx, fee_ppm).unwrap();
    assert_eq!(out, 1_921_203_737_678_824_355_072u128);

    let y_after = y - out;
    assert!(y_after >= WAD);
}

#[test]
fn readme_ex2_quote_out_numbers_match() {
    let x: Wad = 50_000 * WAD;
    let y: Wad = 80_000 * WAD;
    let dy: Wad = 1_850 * WAD;
    let fee_ppm: Ppm = 3_000;

    let num = U256::from(x) * U256::from(dy);
    let den = U256::from(y - dy);
    let dx_net = u256_to_u128_checked(ceil_div_u256(num, den)).unwrap();
    assert_eq!(dx_net, 1_183_621_241_202_815_099_169u128);

    let denom_ppm = U256::from((PPM_SCALE - fee_ppm) as u64);
    let dx_hi = u256_to_u128_checked(ceil_div_u256(
        U256::from(dx_net) * U256::from(PPM_SCALE as u64),
        denom_ppm,
    ))
    .unwrap();
    assert_eq!(dx_hi, 1_187_182_789_571_529_688_234u128);

    let dx_final = swap::get_amount_in(x, y, dy, fee_ppm).unwrap();
    assert_eq!(dx_final, 1_187_182_789_571_529_688_233u128);

    let out = swap::get_amount_out(x, y, dx_final, fee_ppm).unwrap();
    assert!(out >= dy);
    assert!(swap::get_amount_out(x, y, dx_final - 1, fee_ppm).unwrap() < dy);
}

#[test]
fn readme_ex3_slippage_numbers_match() {
    let x: Wad = 50_000 * WAD;
    let y: Wad = 80_000 * WAD;
    let dx: Wad = 1_234 * WAD;
    let fee_ppm: Ppm = 3_000;

    let p_spot = pricing::spot_price_x_in_y(x, y).unwrap();
    assert_eq!(p_spot, 1_600_000_000_000_000_000u128);

    let p_exec = pricing::execution_price_x_to_y(x, y, dx, fee_ppm).unwrap();
    assert_eq!(p_exec, 1_556_891_197_470_684_242u128);

    let slip = pricing::slippage_ppm_x_to_y(x, y, dx, fee_ppm).unwrap();
    assert_eq!(slip, 26_943u32);
}

#[test]
fn readme_ex4_add_liquidity_numbers_match() {
    let x: Wad = 120_000 * WAD;
    let y: Wad = 75_000 * WAD;
    let dx: Wad = 1_000 * WAD;
    let dy: Wad = 450 * WAD;
    let total_shares: Wad = 50_000 * WAD;

    let shares_x = (U256::from(dx) * U256::from(total_shares)) / U256::from(x);
    let shares_y = (U256::from(dy) * U256::from(total_shares)) / U256::from(y);
    assert_eq!(shares_x.as_u128(), 416_666_666_666_666_666_666u128);
    assert_eq!(shares_y.as_u128(), 300_000_000_000_000_000_000u128);

    let minted = liquidity::add_liquidity(x, y, dx, dy, total_shares).unwrap();
    assert_eq!(minted, 300 * WAD);
}
