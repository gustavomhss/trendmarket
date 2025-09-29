use credit_engine_core::amm::{
    guardrails::{div_nearest_even_u256_to_u128, u256_to_u128_checked},
    liquidity,
    pricing,
    swap,
    types::{Ppm, Wad, WAD, PPM_SCALE, U256},
};

fn wad(n: &str) -> Wad {
    n.parse::<u128>().expect("u128") * WAD
}

fn wad_from_str(value: &str) -> Wad {
    let mut parts = value.split('.');
    let int_part = parts.next().expect("int").replace('_', "");
    let frac_part = parts.next().unwrap_or("").replace('_', "");
    if parts.next().is_some() {
        panic!("invalid decimal: {value}");
    }
    let int = if int_part.is_empty() { 0 } else { int_part.parse::<u128>().expect("int") };
    let mut frac = frac_part;
    if frac.len() > 18 {
        panic!("too many decimal places in {value}");
    }
    while frac.len() < 18 {
        frac.push('0');
    }
    let frac_value = if frac.is_empty() {
        0u128
    } else {
        frac.parse::<u128>().expect("frac")
    };
    int
        .checked_mul(WAD)
        .expect("int overflow")
        .checked_add(frac_value)
        .expect("frac overflow")
}

fn ceil_div_u256(n: U256, d: U256) -> U256 {
    (n + (d - U256::from(1u8))) / d
}

fn fee_on_input_debug(dx: Wad, fee_ppm: Ppm) -> Wad {
    if fee_ppm == 0 {
        return 0;
    }
    let n = U256::from(dx) * U256::from(fee_ppm as u64);
    let d = U256::from(PPM_SCALE as u64);
    let fee_u256 = ceil_div_u256(n, d);
    u256_to_u128_checked(fee_u256).expect("fee u128")
}

fn div_nearest_even_debug(n: U256, d: U256) -> Wad {
    div_nearest_even_u256_to_u128(n, d).expect("div nearest even")
}

#[test]
fn compute_swap_get_amount_out() {
    let x = wad("125000000");
    let y = wad("83000000");
    let dx = wad_from_str("275000.432109");
    let fee_ppm: Ppm = 3_000; // 30 bps

    let dx_fee = fee_on_input_debug(dx, fee_ppm);
    let dx_net = dx - dx_fee;
    let x1 = x + dx_net;
    let k = U256::from(x) * U256::from(y);
    let y_star = div_nearest_even_debug(k, U256::from(x1));
    let out = swap::get_amount_out(x, y, dx, fee_ppm).expect("swap out");
    let y1 = y - out;

    assert_eq!(dx, 275000432109000000000000u128);
    assert_eq!(dx_fee, 825001296327000000000u128);
    assert_eq!(dx_net, 274175430812673000000000u128);
    assert_eq!(x1, 125274175430812673000000000u128);
    assert_eq!(y_star, 82818345954549746632789137u128);
    assert_eq!(out, 181654045450253367210863u128);
    assert_eq!(y1, 82818345954549746632789137u128);
}

#[test]
fn compute_swap_get_amount_in() {
    let x = wad("48000000");
    let y = wad("12100000");
    let dy = wad_from_str("525500.875012");
    let fee_ppm: Ppm = 5_000; // 50 bps scenario to test rounding ceil

    let num = U256::from(x) * U256::from(dy);
    let y_minus_dy = y.checked_sub(dy).expect("dy<y");
    let den = U256::from(y_minus_dy);
    let dx_net_est = u256_to_u128_checked(ceil_div_u256(num, den)).expect("dx_net");
    let denom_ppm = (PPM_SCALE as u64) - (fee_ppm as u64);
    let dx_gross_guess = u256_to_u128_checked(ceil_div_u256(
        U256::from(dx_net_est) * U256::from(PPM_SCALE as u64),
        U256::from(denom_ppm),
    ))
    .expect("dx_gross_guess");

    let dx = swap::get_amount_in(x, y, dy, fee_ppm).expect("swap in");

    assert_eq!(dy, 525500875012000000000000u128);
    assert_eq!(dx_net_est, 2179277196204561579795744u128);
    assert_eq!(dx_gross_guess, 2190228337894031738488185u128);
    assert_eq!(dx, 2190228337894031738488183u128);
}

#[test]
fn compute_pricing_min_out_with_tolerance() {
    let x = wad("90500000");
    let y = wad("64250000");
    let dx = wad_from_str("1200000.125987");
    let fee_ppm: Ppm = 3_000;
    let tolerance_ppm: Ppm = 950_000; // 95.0% próximo do limite máximo

    let out = swap::get_amount_out(x, y, dx, fee_ppm).expect("amount out");
    let tol = tolerance_ppm.min(PPM_SCALE);
    let factor = (PPM_SCALE - tol) as u64;
    let min_out = pricing::min_out_with_tolerance(x, y, dx, fee_ppm, tolerance_ppm)
        .expect("min out");

    assert_eq!(dx, 1200000125987000000000000u128);
    assert_eq!(out, 838295810577985881513049u128);
    assert_eq!(factor, 50_000u64);
    assert_eq!(min_out, 41914790528899294075652u128);
}

#[test]
fn compute_pricing_slippage_ppm() {
    let x = wad("32750000");
    let y = wad("112900000");
    let dx = wad_from_str("950000.784321");
    let fee_ppm: Ppm = 3_000;

    let spot = pricing::spot_price_x_in_y(x, y).expect("spot");
    let exec = pricing::execution_price_x_to_y(x, y, dx, fee_ppm).expect("exec");
    let slip = pricing::slippage_ppm_x_to_y(x, y, dx, fee_ppm).expect("slippage");

    let num = (U256::from(spot) - U256::from(exec)) * U256::from(PPM_SCALE as u64);
    let raw = div_nearest_even_u256_to_u128(num, U256::from(spot)).expect("ratio");

    assert_eq!(spot, 3447328244274809160u128);
    assert_eq!(exec, 3340380340412448601u128);
    assert_eq!(raw, 31023u128);
    assert_eq!(slip, 31023u32);
}

#[test]
fn compute_liquidity_add_liquidity() {
    let x = wad("78500000");
    let y = wad("91500000");
    let total_shares = wad("152500000");
    let dx_add = wad_from_str("2500000.458765");
    let dy_add = wad_from_str("2950000.876543");

    let minted_x = (U256::from(dx_add) * U256::from(total_shares)) / U256::from(x);
    let minted_y = (U256::from(dy_add) * U256::from(total_shares)) / U256::from(y);
    let minted_x = u256_to_u128_checked(minted_x).expect("minted_x");
    let minted_y = u256_to_u128_checked(minted_y).expect("minted_y");
    let minted = liquidity::add_liquidity(x, y, dx_add, dy_add, total_shares).expect("minted");

    assert_eq!(dx_add, 2500000458765000000000000u128);
    assert_eq!(dy_add, 2950000876543000000000000u128);
    assert_eq!(minted_x, 4856688789320541401273885u128);
    assert_eq!(minted_y, 4916668127571666666666666u128);
    assert_eq!(minted, 4856688789320541401273885u128);
}
