//! Funções puras de swap (CPMM x·y=k), conforme ADR-0001/0002.
//! - amount_out: trocando X→Y com taxa sobre o input
//! - amount_in_for_out: menor dx bruto que entrega ao menos dy em Y

use super::errors::AmmError;
use super::guardrails::{
    checked_add, checked_sub, div_nearest_even_u256_to_u128, ensure_nonzero, ensure_reserves,
};
use super::types::{U256, Ppm, Wad, PPM_SCALE, MIN_RESERVE};

#[inline]
fn ceil_div_u256(n: U256, d: U256) -> U256 {
    // (n + d - 1) / d, assumindo d>0
    (n + (d - U256::from(1u8))) / d
}

#[inline]
fn fee_on_input_ceil(dx: Wad, fee_ppm: Ppm) -> Wad {
    if fee_ppm == 0 { return 0; }
    let n = U256::from(dx) * U256::from(fee_ppm as u64);
    let d = U256::from(PPM_SCALE as u64);
    ceil_div_u256(n, d).as_u128()
}

/// Calcula o `amount_out` ao enviar `dx` do ativo X e receber Y (X→Y).
/// Política de arredondamento:
/// - fee(input): ceil
/// - divisão interna k/x': nearest (ties-to-even)
/// - fronteira (out): floor via subtração inteira (y - y*)
pub fn get_amount_out(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    ensure_nonzero(dx)?;

    // taxa sobre o input
    let dx_fee = fee_on_input_ceil(dx, fee_ppm);
    let dx_net = dx.checked_sub(dx_fee).ok_or(AmmError::Overflow)?;
    if dx_net == 0 { return Err(AmmError::InputTooSmall); }

    // x' = x + dx_net (checado)
    let x1 = checked_add(x, dx_net)?;

    // y* = (x*y)/x' com nearest-even em 256 bits → u128
    let k = U256::from(x) * U256::from(y);
    let y_star = div_nearest_even_u256_to_u128(k, U256::from(x1))?;

    // out = floor(y - y*)
    let out = y.checked_sub(y_star).ok_or(AmmError::Overflow)?;

    // y' >= min_reserve
    let y1 = y.checked_sub(out).ok_or(AmmError::Overflow)?;
    if y1 < MIN_RESERVE { return Err(AmmError::MinReserveBreached); }

    Ok(out)
}

/// Calcula o **menor dx bruto** tal que `get_amount_out(x,y,dx,fee) ≥ dy`.
/// Passos (ADR-0002):
/// 1) dx_net = ceil( x * dy / (y - dy) )
/// 2) dx_bruto = ceil( dx_net * 1e6 / (1e6 - fee_ppm) )
/// 3) Ajuste se devido ao ceil da taxa dx_net efetivo ficar curto (dx++)
pub fn get_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    ensure_nonzero(dy)?;

    // Não pode esvaziar o pool além do mínimo
    if dy >= y.checked_sub(MIN_RESERVE).ok_or(AmmError::Overflow)? {
        return Err(AmmError::MinReserveBreached);
    }

    // 1) dx_net = ceil( x * dy / (y - dy) )
    let num = U256::from(x) * U256::from(dy);
    let den = U256::from(y.checked_sub(dy).ok_or(AmmError::Overflow)?);
    let dx_net_u256 = ceil_div_u256(num, den);
    let dx_net = dx_net_u256.as_u128();

    // 2) bruto a partir do net (cuidando do caso fee_ppm=1e6)
    let denom_ppm = (PPM_SCALE as u64) - (fee_ppm as u64);
    if denom_ppm == 0 { return Err(AmmError::InputTooSmall); }
    let dx_bruto_u256 = ceil_div_u256(
        U256::from(dx_net) * U256::from(PPM_SCALE as u64),
        U256::from(denom_ppm),
    );
    let mut dx = dx_bruto_u256.as_u128();

    // 3) correção por arredondamento da taxa: garantir dx_net efetivo suficiente
    loop {
        let fee = fee_on_input_ceil(dx, fee_ppm);
        let net = dx.checked_sub(fee).ok_or(AmmError::Overflow)?;
        if net >= dx_net { break; }
        dx = dx.checked_add(1).ok_or(AmmError::Overflow)?;
    }

    // sanity check: aplicar get_amount_out deve entregar >= dy
    let out = get_amount_out(x, y, dx, fee_ppm)?;
    if out < dy {
        // por segurança, aumenta 1 até satisfazer; limite prático é curto
        let mut dx2 = dx;
        while get_amount_out(x, y, dx2, fee_ppm)? < dy {
            dx2 = dx2.checked_add(1).ok_or(AmmError::Overflow)?;
        }
        return Ok(dx2);
    }
    Ok(dx)
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
    fn t_out_symmetric_no_fee() {
        let (x, y, dx) = (1_000_000u128, 1_000_000u128, 10_000u128);
        let out = get_amount_out(x, y, dx, FEE0).unwrap();
        assert_eq!(out, 9_900);
        // y' = 990_100 >= MIN_RESERVE (1)
        let y1 = y - out;
        assert!(y1 >= MIN_RESERVE);
    }

    #[test]
    fn t_out_symmetric_with_fee() {
        let (x, y, dx) = (1_000_000u128, 1_000_000u128, 10_000u128);
        let out = get_amount_out(x, y, dx, FEE3).unwrap();
        assert_eq!(out, 9_870);
    }

    #[test]
    fn t_in_for_target_out_with_fee() {
        let (x, y, dy) = (1_000_000u128, 1_000_000u128, 9_870u128);
        let dx = get_amount_in(x, y, dy, FEE3).unwrap();
        assert_eq!(dx, 10_000);
        let out = get_amount_out(x, y, dx, FEE3).unwrap();
        assert!(out >= dy);
    }

    #[test]
    fn t_out_asymmetric_no_fee() {
        let (x, y, dx) = (1_000u128, 1_000_000_000u128, 100u128);
        let out = get_amount_out(x, y, dx, FEE0).unwrap();
        assert_eq!(out, 90_909_090);
    }

    #[test]
    fn t_dx_zero_rejected() {
        let err = get_amount_out(1_000_000, 1_000_000, 0, FEE0).unwrap_err();
        assert_eq!(err, AmmError::ZeroAmount);
    }

    #[test]
    fn t_dx_net_zero_due_fee_rejected() {
        let err = get_amount_out(5_000_000, 4_000_000, 1, FEE3).unwrap_err();
        assert_eq!(err, AmmError::InputTooSmall);
    }

    #[test]
    fn t_dy_too_large_rejected() {
        let err = get_amount_in(1_000_000, 1_000_000, 1_000_000 - MIN_RESERVE, FEE0).unwrap_err();
        assert_eq!(err, AmmError::MinReserveBreached);
    }

    #[test]
    fn t_zero_reserve_rejected() {
        let err = get_amount_out(0, 1_000_000, 10, FEE0).unwrap_err();
        assert_eq!(err, AmmError::ZeroReserve);
    }

    #[test]
    fn t_invariant_with_fee_k_monotonic() {
        let (x, y, dx) = (1_000_000u128, 1_000_000u128, 10_000u128);
        let k0 = U256::from(x) * U256::from(y);
        let out = get_amount_out(x, y, dx, FEE3).unwrap();
        let x1 = x + (dx - fee_on_input_ceil(dx, FEE3));
        let y1 = y - out;
        let k1 = U256::from(x1) * U256::from(y1);
        assert!(k1 >= k0);
    }

    #[test]
    fn t_small_values_min_reserve_guard() {
        // y muito pequena pós-swap quebra min_reserve
        let (x, y, dx) = (MIN_RESERVE + 10, MIN_RESERVE + 5, 1);
        let err = get_amount_out(x, y, dx, FEE0).unwrap_err();
        assert_eq!(err, AmmError::MinReserveBreached);
    }
}
