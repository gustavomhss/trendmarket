//! Utilitários para UI/roteadores: spot price, preço de execução, slippage e cotações.
//! Baseados nos ADRs 0001/0002 e nas funções puras de `swap.rs`.

use super::errors::AmmError;
use super::guardrails::{
    div_nearest_even_u256, div_nearest_even_u256_to_u128, ensure_nonzero, ensure_reserves, u256_to_u128_checked,
};
use super::swap::{get_amount_in, get_amount_out};
use super::types::{U256, Ppm, Wad, PPM_SCALE, WAD, MIN_RESERVE};

#[inline]
fn ceil_div_u256(n: U256, d: U256) -> U256 { (n + (d - U256::from(1u8))) / d }

// --------- Spot price ---------
/// Preço à vista de 1 X em Y (dy/dx infinitesimal): **p = y/x** (em WAD)
pub fn spot_price_x_in_y(x: Wad, y: Wad) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    let n = U256::from(y) * U256::from(WAD);
    div_nearest_even_u256_to_u128(n, U256::from(x))
}

/// Preço à vista de 1 Y em X (dx/dy infinitesimal): **p = x/y** (em WAD)
pub fn spot_price_y_in_x(x: Wad, y: Wad) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    let n = U256::from(x) * U256::from(WAD);
    div_nearest_even_u256_to_u128(n, U256::from(y))
}

// --------- Execução e slippage ---------
/// Preço efetivo (execução) da troca X→Y para um `dx` **bruto** (inclui taxa): **p_exec = out/dx** (em WAD)
pub fn execution_price_x_to_y(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    ensure_nonzero(dx)?;
    let out = get_amount_out(x, y, dx, fee_ppm)?;
    let n = U256::from(out) * U256::from(WAD);
    div_nearest_even_u256_to_u128(n, U256::from(dx))
}

/// Slippage relativo em **PPM** comparando `p_exec` vs `spot` (sempre ≥0):
/// slippage_ppm = ((spot - p_exec) / spot) * 1e6
pub fn slippage_ppm_x_to_y(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<Ppm, AmmError> {
    let spot = spot_price_x_in_y(x, y)?;           // WAD
    let exec = execution_price_x_to_y(x, y, dx, fee_ppm)?; // WAD
    if exec >= spot { return Ok(0); }
    let num = (U256::from(spot) - U256::from(exec)) * U256::from(PPM_SCALE as u64);
    let den = U256::from(spot);
    let q = div_nearest_even_u256(num, den)?; // U256
    let q128 = u256_to_u128_checked(q)?;
    let mut ppm = if q128 > u128::from(u32::MAX) { u32::MAX } else { q128 as u32 };
    if ppm > PPM_SCALE { ppm = PPM_SCALE; }
    Ok(ppm)
}

// --------- Cotas com tolerância de slippage ---------
/// Retorna **min_out** aceito pela UI para X→Y considerando `slippage_tolerance_ppm` (0..1e6)
/// min_out = floor( out * (1 - tol) )
pub fn min_out_with_tolerance(
    x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm, slippage_tolerance_ppm: Ppm,
) -> Result<Wad, AmmError> {
    let out = get_amount_out(x, y, dx, fee_ppm)?;
    let tol = if slippage_tolerance_ppm > PPM_SCALE { PPM_SCALE } else { slippage_tolerance_ppm } as u64;
    let factor = (PPM_SCALE as u64) - tol; // (1 - tol)
    let n = U256::from(out) * U256::from(factor);
    let q = n / U256::from(PPM_SCALE as u64); // floor
    Ok(q.as_u128())
}

/// Retorna **max_in** aceito pela UI para atingir `dy` com tolerância `slippage_tolerance_ppm` (0..1e6)
/// max_in = ceil( dx * (1 + tol) )
pub fn max_in_with_tolerance(
    x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm, slippage_tolerance_ppm: Ppm,
) -> Result<Wad, AmmError> {
    let dx = get_amount_in(x, y, dy, fee_ppm)?;
    let tol = if slippage_tolerance_ppm > PPM_SCALE { PPM_SCALE } else { slippage_tolerance_ppm } as u64;
    let factor = (PPM_SCALE as u64) + tol; // (1 + tol)
    let n = U256::from(dx) * U256::from(factor);
    let q = ceil_div_u256(n, U256::from(PPM_SCALE as u64)); // ceil
    Ok(q.as_u128())
}

// -------------------------
// TESTES
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;

    const FEE0: Ppm = 0;
    const FEE3: Ppm = 3000; // 0,30%

    #[test]
    fn t_spot_prices_basic() {
        let (x, y) = (1_000_000u128, 2_000_000u128);
        let p_xy = spot_price_x_in_y(x, y).unwrap(); // 2.0
        let p_yx = spot_price_y_in_x(x, y).unwrap(); // 0.5
        assert_eq!(p_xy, 2 * WAD);
        assert_eq!(p_yx, WAD / 2);
    }

    #[test]
    fn t_slippage_no_fee_vs_fee() {
        let (x, y, dx) = (1_000_000u128, 1_000_000u128, 10_000u128);
        let s0 = slippage_ppm_x_to_y(x, y, dx, FEE0).unwrap();
        let s3 = slippage_ppm_x_to_y(x, y, dx, FEE3).unwrap();
        assert_eq!(s0, 10_000);  // 1.0%
        assert_eq!(s3, 13_000);  // 1.3% (impacto + taxa)
    }

    #[test]
    fn t_min_out_with_tolerance() {
        let (x, y, dx) = (1_000_000u128, 1_000_000u128, 10_000u128);
        // out (com taxa) = 9_870 ; tol = 0,5% -> min_out = floor(9870 * 0.995) = 9820
        let min_out = min_out_with_tolerance(x, y, dx, FEE3, 5_000).unwrap();
        assert_eq!(min_out, 9_820);
    }

    #[test]
    fn t_max_in_with_tolerance() {
        let (x, y, dy) = (1_000_000u128, 1_000_000u128, 9_870u128);
        // dx (com taxa) = 10_000 ; tol = 0,5% -> max_in = ceil(10000 * 1.005) = 10050
        let max_in = max_in_with_tolerance(x, y, dy, FEE3, 5_000).unwrap();
        assert_eq!(max_in, 10_050);
    }

    #[test]
    fn t_exec_price_matches_ratio_out_over_dx() {
        let (x, y, dx) = (1_000_000u128, 1_000_000u128, 10_000u128);
        let out = get_amount_out(x, y, dx, FEE0).unwrap();
        let p_exec = execution_price_x_to_y(x, y, dx, FEE0).unwrap();
        let p_exec_check = (U256::from(out) * U256::from(WAD)) / U256::from(dx);
        assert_eq!(p_exec as u128, p_exec_check.as_u128());
    }

    #[test]
    fn t_safety_invalid_inputs() {
        // reservas inválidas
        assert!(spot_price_x_in_y(0, MIN_RESERVE).is_err());
        // dx zero na execução
        assert!(execution_price_x_to_y(MIN_RESERVE, MIN_RESERVE, 0, FEE0).is_err());
    }
}
