//! Funções puras de swap (CPMM x·y=k), conforme ADR-0001/0002.
//! - get_amount_out: trocando X→Y com taxa sobre o input
//! - get_amount_in: menor dx bruto que entrega ao menos dy em Y (minimalidade garantida)

use super::errors::AmmError;
use super::guardrails::{
    checked_add,
    div_nearest_even_u256_to_u128,
    ensure_nonzero,
    ensure_reserves,
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

    // y* = round_nearest_even((x*y)/x') em 256 bits → u128
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
/// Estratégia: usa a fórmula fechada para chutar um **upper bound** e depois faz
/// **busca binária** em cima de `get_amount_out` para garantir **minimalidade**.
/// Passos (ADR-0002, com robustez a arredondamentos):
/// 1) dx_net = ceil( x * dy / (y - dy) )
/// 2) dx_gross_guess = ceil( dx_net * 1e6 / (1e6 - fee_ppm) )
/// 3) expande `hi` até `out(hi) ≥ dy` (se necessário)
/// 4) busca binária no menor `dx` com `out ≥ dy`
pub fn get_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    ensure_nonzero(dy)?;

    // Não pode esvaziar o pool além do mínimo
    if dy >= y.checked_sub(MIN_RESERVE).ok_or(AmmError::Overflow)? {
        return Err(AmmError::MinReserveBreached);
    }

    // -------- upper bound (chute via ADR-0002) --------
    // dx_net = ceil( x * dy / (y - dy) )
    let num = U256::from(x) * U256::from(dy);
    let den = U256::from(y.checked_sub(dy).ok_or(AmmError::Overflow)?);
    let dx_net = ceil_div_u256(num, den).as_u128();

    // dx_gross = ceil( dx_net * 1e6 / (1e6 - fee) )
    let denom_ppm = (PPM_SCALE as u64).checked_sub(fee_ppm as u64).ok_or(AmmError::InputTooSmall)?;
    if denom_ppm == 0 { return Err(AmmError::InputTooSmall); }
    let mut hi = ceil_div_u256(
        U256::from(dx_net) * U256::from(PPM_SCALE as u64),
        U256::from(denom_ppm),
    ).as_u128();
    if hi == 0 { hi = 1; }

    // garante que `hi` satisfaz (expande se necessário)
    loop {
        let out_hi = get_amount_out(x, y, hi, fee_ppm).unwrap_or(0);
        if out_hi >= dy { break; }
        hi = hi.checked_mul(2).ok_or(AmmError::Overflow)?;
    }

    // -------- busca binária: menor dx com out ≥ dy --------
    let mut lo: Wad = 0;
    while lo < hi {
        // mid seguro: lo + (hi-lo)/2
        let mut mid = lo + ((hi - lo) >> 1);
        if mid == 0 { mid = 1; } // dx=0 nunca serve

        let out_mid = match get_amount_out(x, y, mid, fee_ppm) {
            Ok(v) => v,
            Err(_) => 0, // por robustez: trate erro como insuficiente
        };

        if out_mid >= dy {
            // satisfaz → tenta menor
            hi = mid;
        } else {
            // não satisfaz → precisa mais
            lo = mid + 1;
        }
    }

    Ok(hi)
}

// -------------------------
// TESTES
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::amm::types::{MIN_RESERVE, U256, Ppm, WAD};

    const FEE0: Ppm = 0;
    const FEE3: Ppm = 3000; // 0,30%

    #[test]
    fn t_out_symmetric_no_fee() {
        let (x, y, dx) = (1_000_000u128 * WAD, 1_000_000u128 * WAD, 10_000u128 * WAD);
        let out = get_amount_out(x, y, dx, FEE0).unwrap();

        // esperado via y* arredondado igual ao core
        let x1 = x + dx;
        let k = U256::from(x) * U256::from(y);
        let y_star = crate::amm::guardrails::div_nearest_even_u256_to_u128(k, U256::from(x1)).unwrap();
        assert_eq!(out, y - y_star);

        // y' permanece >= mínimo
        let y1 = y - out;
        assert!(y1 >= MIN_RESERVE);
    }

    #[test]
    fn t_out_symmetric_with_fee() {
        let (x, y, dx) = (1_000_000u128 * WAD, 1_000_000u128 * WAD, 10_000u128 * WAD);
        let out = get_amount_out(x, y, dx, FEE3).unwrap();

        let dx_fee = super::fee_on_input_ceil(dx, FEE3);
        let x1 = x + (dx - dx_fee);
        let k = U256::from(x) * U256::from(y);
        let y_star = crate::amm::guardrails::div_nearest_even_u256_to_u128(k, U256::from(x1)).unwrap();
        let expected = y - y_star;
        assert_eq!(out, expected);
    }

    #[test]
    fn t_in_for_target_out_with_fee_minimal() {
        let (x, y, dy) = (1_000_000u128 * WAD, 1_000_000u128 * WAD, 9_870u128 * WAD);
        let dx = get_amount_in(x, y, dy, FEE3).unwrap();
        // minimalidade: dx-1 não alcança
        if dx > 0 {
            let out_prev = get_amount_out(x, y, dx - 1, FEE3).unwrap_or(0);
            assert!(out_prev < dy);
        }
        let out = get_amount_out(x, y, dx, FEE3).unwrap();
        assert!(out >= dy);
    }

    #[test]
    fn t_out_asymmetric_no_fee() {
        let (x, y, dx) = (1_000u128 * WAD, 1_000_000_000u128 * WAD, 100u128 * WAD);
        let out = get_amount_out(x, y, dx, FEE0).unwrap();

        let x1 = x + dx;
        let k = U256::from(x) * U256::from(y);
        let y_star = crate::amm::guardrails::div_nearest_even_u256_to_u128(k, U256::from(x1)).unwrap();
        let expected = y - y_star;
        assert_eq!(out, expected);
    }

    #[test]
    fn t_dx_zero_rejected() {
        let err = get_amount_out(1_000_000u128 * WAD, 1_000_000u128 * WAD, 0, FEE0).unwrap_err();
        assert_eq!(err, AmmError::ZeroAmount);
    }

    #[test]
    fn t_dx_net_zero_due_fee_rejected() {
        // dx=1 wei e fee>0 ⇒ fee=1 ⇒ dx_net=0
        let err = get_amount_out(5_000_000u128 * WAD, 4_000_000u128 * WAD, 1, FEE3).unwrap_err();
        assert_eq!(err, AmmError::InputTooSmall);
    }

    #[test]
    fn t_dy_too_large_rejected() {
        // pedir dy>=y-MIN_RESERVE viola guarda
        let (x, y) = (MIN_RESERVE + 2_000, MIN_RESERVE + 1_000);
        let dy = y - MIN_RESERVE;
        let err = get_amount_in(x, y, dy, FEE0).unwrap_err();
        assert_eq!(err, AmmError::MinReserveBreached);
    }

    #[test]
    fn t_zero_reserve_rejected() {
        let err = get_amount_out(0, 1_000_000u128 * WAD, 10 * WAD, FEE0).unwrap_err();
        assert_eq!(err, AmmError::ZeroReserve);
    }

    #[test]
    fn t_small_values_min_reserve_guard() {
        // y = MIN_RESERVE e dx grande ⇒ y' cai abaixo do mínimo ⇒ erro
        let (x, y, dx) = (MIN_RESERVE + 10, MIN_RESERVE, MIN_RESERVE);
        let err = get_amount_out(x, y, dx, FEE0).unwrap_err();
        assert_eq!(err, AmmError::MinReserveBreached);
    }
}
