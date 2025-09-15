//! Utilitários para UI/roteadores: spot price, preço de execução, slippage e cotações.
//! Baseados nos ADRs 0001/0002 e nas funções puras de `swap.rs`.

use super::errors::AmmError;
use super::guardrails::{
    div_nearest_even_u256, div_nearest_even_u256_to_u128, ensure_nonzero, ensure_reserves, u256_to_u128_checked,
};
use super::swap::{get_amount_in, get_amount_out};
use super::types::{U256, Ppm, Wad, PPM_SCALE, WAD};

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
// TESTES (WAD-scaled)
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::amm::types::MIN_RESERVE;

    const FEE0: Ppm = 0;
    const FEE3: Ppm = 3000; // 0,30%

    #[test]
    fn t_spot_prices_basic() {
        let (x, y) = (1_000_000u128*WAD, 2_000_000u128*WAD);
        let p_xy = spot_price_x_in_y(x, y).unwrap(); // 2.0
        let p_yx = spot_price_y_in_x(x, y).unwrap(); // 0.5
        assert_eq!(p_xy, 2 * WAD);
        assert_eq!(p_yx, WAD / 2);
    }

    #[test]
    fn t_slippage_no_fee_vs_fee() {
        let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD);
        let s0 = slippage_ppm_x_to_y(x, y, dx, FEE0).unwrap();
        let s3 = slippage_ppm_x_to_y(x, y, dx, FEE3).unwrap();
        // Janela bem apertada em torno de ~1.00% e ~1.30% para acomodar nearest-even
        assert!((9_800..=10_200).contains(&s0), "s0={}ppm (esperado ~10_000ppm)", s0);
        assert!((12_800..=13_200).contains(&s3), "s3={}ppm (esperado ~13_000ppm)", s3);
    }

    #[test]
    fn t_min_out_with_tolerance() {
        let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD);
        let min_out = min_out_with_tolerance(x, y, dx, FEE3, 5_000).unwrap();

        // Esperado = floor( out * (1 - tol) ), usando o out real
        let out = get_amount_out(x, y, dx, FEE3).unwrap();
        let factor = (PPM_SCALE as u64) - 5_000u64;
        let expected = (U256::from(out) * U256::from(factor)) / U256::from(PPM_SCALE as u64);
        assert_eq!(min_out, expected.as_u128());
    }

    #[test]
    fn t_max_in_with_tolerance() {
        let (x, y, dy) = (1_000_000u128*WAD, 1_000_000u128*WAD, 9_870u128*WAD);
        let max_in = max_in_with_tolerance(x, y, dy, FEE3, 5_000).unwrap();

        // Esperado = ceil( dx_core * (1 + tol) ), usando o dx_core real
        let dx_core = get_amount_in(x, y, dy, FEE3).unwrap();
        let factor = (PPM_SCALE as u64) + 5_000u64;
        let expected = super::ceil_div_u256(
            U256::from(dx_core) * U256::from(factor),
            U256::from(PPM_SCALE as u64),
        )
        .as_u128();

        assert_eq!(max_in, expected);
    }

    #[test]
    fn t_exec_price_matches_ratio_out_over_dx() {
        let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD);
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