//! Mapeamento entre condições de domínio e códigos de erro do AMM.
use crate::amm::error::AmmError;
use crate::amm::error_catalog::AmmErrorCode;

/// Determina o código de erro para um swap a partir dos inputs brutos.
pub fn from_swap_inputs(
    dx: u128,
    reserves: (u128, u128),
    min_reserve: u128,
    fee_bps: u32,
) -> Option<AmmErrorCode> {
    if dx == 0 {
        return Some(AmmErrorCode::ZeroAmount);
    }
    if reserves.0 == 0 || reserves.1 == 0 {
        return Some(AmmErrorCode::ZeroReserve);
    }
    if reserves.0 <= min_reserve || reserves.1 <= min_reserve {
        return Some(AmmErrorCode::MinReserveBreached);
    }
    let fee_bps_u128 = u128::from(fee_bps);
    let fee = match dx.checked_mul(fee_bps_u128) {
        Some(product) => product / 10_000u128,
        None => return Some(AmmErrorCode::OverflowNumeric),
    };
    let dx_eff = match dx.checked_sub(fee) {
        Some(val) => val,
        None => return Some(AmmErrorCode::OverflowNumeric),
    };
    if dx_eff == 0 {
        return Some(AmmErrorCode::EffectiveInputZero);
    }
    if reserves.0.checked_add(dx_eff).is_none() {
        return Some(AmmErrorCode::OverflowNumeric);
    }
    None
}

/// Constrói um [`AmmError`] diretamente de um código.
pub fn to_error(code: AmmErrorCode) -> AmmError {
    AmmError::new(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_zero_amount() {
        assert_eq!(
            from_swap_inputs(0, (1, 1), 1, 0),
            Some(AmmErrorCode::ZeroAmount)
        );
    }

    #[test]
    fn detects_zero_reserve() {
        assert_eq!(
            from_swap_inputs(1, (0, 1), 1, 0),
            Some(AmmErrorCode::ZeroReserve)
        );
    }

    #[test]
    fn detects_min_reserve() {
        assert_eq!(
            from_swap_inputs(1, (1, 2), 2, 0),
            Some(AmmErrorCode::MinReserveBreached)
        );
    }

    #[test]
    fn detects_effective_zero() {
        assert_eq!(
            from_swap_inputs(10, (10, 10), 1, 10_000),
            Some(AmmErrorCode::EffectiveInputZero)
        );
    }

    #[test]
    fn detects_overflow() {
        assert_eq!(
            from_swap_inputs(u128::MAX, (u128::MAX, 10), 1, 10_000),
            Some(AmmErrorCode::OverflowNumeric)
        );
    }

    #[test]
    fn ok_path() {
        assert_eq!(from_swap_inputs(10, (100, 100), 1, 30), None);
    }
}