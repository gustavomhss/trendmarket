//! Liquidez (CPMM): mint inicial, add e remove de shares.
//! Políticas (ADR-0001):
//! - shares_mint: **floor**
//! - amounts_out em burn: **floor**
//! - validações de mínimos/overflow via guardrails

use super::errors::AmmError;
use super::guardrails::{
    checked_add, checked_sub, mul_u128_to_u256, u256_to_u128_checked,
    ensure_nonzero, ensure_reserves,
};
use super::types::{U256, Wad, MIN_RESERVE};

#[inline]
fn isqrt_u256(n: U256) -> U256 {
    if n.is_zero() { return U256::from(0u8); }
    let mut low = U256::from(0u8);
    let mut high = n;
    while low < high {
        let mid = (low + high + U256::from(1u8)) >> 1; // ceil((low+high)/2)
        // evitar overflow: mid*mid <= n  <=>  mid <= n/mid
        if mid <= n / mid { low = mid; } else { high = mid - U256::from(1u8); }
    }
    low
}

/// Mint **inicial** de shares: `floor(sqrt(x*y))`.
/// Requer reservas válidas (>= MIN_RESERVE).
pub fn initial_mint(x: Wad, y: Wad) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    let k = mul_u128_to_u256(x, y);
    let s = isqrt_u256(k);
    let shares = u256_to_u128_checked(s)?;
    if shares == 0 { return Err(AmmError::InputTooSmall); }
    Ok(shares)
}

/// Mint em pool existente (proporcional). Retorna **shares mintados** (floor).
/// Fórmula: `shares = floor( min(dx * S / x , dy * S / y) )`, onde `S=total_shares`.
pub fn add_liquidity(x: Wad, y: Wad, dx: Wad, dy: Wad, total_shares: Wad) -> Result<Wad, AmmError> {
    ensure_reserves(x, y)?;
    ensure_nonzero(dx)?;
    ensure_nonzero(dy)?;
    if total_shares == 0 { return Err(AmmError::Overflow); } // uso errado: pool não deveria estar vazia

    let s = U256::from(total_shares);
    let sx = (U256::from(dx) * s) / U256::from(x); // floor
    let sy = (U256::from(dy) * s) / U256::from(y);
    let mint = if sx < sy { sx } else { sy };
    let shares = u256_to_u128_checked(mint)?;
    if shares == 0 { return Err(AmmError::InputTooSmall); }

    // pós-condição de segurança: reservas só aumentam
    let _x1 = checked_add(x, dx)?;
    let _y1 = checked_add(y, dy)?;
    Ok(shares)
}

/// Burn de shares (proporcional). Retorna (amount_x, amount_y) com **floor**.
/// Fórmulas: `x_out = floor(x * burn / S)`, idem para `y`.
/// Garante que reservas remanescentes `x'`,`y'` ficam >= MIN_RESERVE.
pub fn remove_liquidity(x: Wad, y: Wad, burn_shares: Wad, total_shares: Wad) -> Result<(Wad, Wad), AmmError> {
    ensure_reserves(x, y)?;
    ensure_nonzero(burn_shares)?;
    if total_shares == 0 { return Err(AmmError::Overflow); }
    if burn_shares > total_shares { return Err(AmmError::Overflow); }

    let bx = (U256::from(x) * U256::from(burn_shares)) / U256::from(total_shares);
    let by = (U256::from(y) * U256::from(burn_shares)) / U256::from(total_shares);
    let x_out = u256_to_u128_checked(bx)?;
    let y_out = u256_to_u128_checked(by)?;

    // não permitir esvaziar abaixo do mínimo
    let x1 = checked_sub(x, x_out)?;
    let y1 = checked_sub(y, y_out)?;
    if x1 < MIN_RESERVE || y1 < MIN_RESERVE { return Err(AmmError::MinReserveBreached); }

    if x_out == 0 && y_out == 0 { return Err(AmmError::InputTooSmall); }
    Ok((x_out, y_out))
}

// -------------------------
// TESTES (WAD-scaled)
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::amm::types::WAD;

    #[test]
    fn t_initial_mint_symmetrical() {
        let (x, y) = (1_000_000u128*WAD, 1_000_000u128*WAD);
        let s = initial_mint(x, y).unwrap();
        assert_eq!(s, 1_000_000u128*WAD);
    }

    #[test]
    fn t_initial_mint_min_reserve_guard() {
        let (x, y) = (MIN_RESERVE - 1, MIN_RESERVE);
        let err = initial_mint(x, y).unwrap_err();
        assert_eq!(err, AmmError::MinReserveBreached);
    }

    #[test]
    fn t_add_liquidity_proportional_sym() {
        let (x, y, s) = (1_000_000u128*WAD, 1_000_000u128*WAD, 1_000_000u128*WAD);
        let (dx, dy) = (100_000u128*WAD, 100_000u128*WAD);
        let mint = add_liquidity(x, y, dx, dy, s).unwrap();
        assert_eq!(mint, 100_000u128*WAD);
    }

    #[test]
    fn t_add_liquidity_min_by_y() {
        let (x, y, s) = (1_000_000u128*WAD, 1_000_000u128*WAD, 1_000_000u128*WAD);
        let (dx, dy) = (200_000u128*WAD, 100_000u128*WAD); // y limita
        let mint = add_liquidity(x, y, dx, dy, s).unwrap();
        assert_eq!(mint, 100_000u128*WAD);
    }

    #[test]
    fn t_add_liquidity_too_small() {
        // S pequeno o bastante para floor(...) == 0 e falhar com InputTooSmall
        let (x, y, s) = (1_000_000u128*WAD, 1_000_000u128*WAD, 100u128);
        let (dx, dy) = (1u128, 1u128);
        let err = add_liquidity(x, y, dx, dy, s).unwrap_err();
        assert_eq!(err, AmmError::InputTooSmall);
    }

    #[test]
    fn t_remove_liquidity_10_percent() {
        let (x, y, s) = (1_000_000u128*WAD, 1_000_000u128*WAD, 1_000_000u128*WAD);
        let burn = 100_000u128*WAD; // 10%
        let (xo, yo) = remove_liquidity(x, y, burn, s).unwrap();
        assert_eq!((xo, yo), (100_000u128*WAD, 100_000u128*WAD));
        let (x1, y1) = (x - xo, y - yo);
        assert!(x1 >= MIN_RESERVE && y1 >= MIN_RESERVE);
    }

    #[test]
    fn t_remove_liquidity_burn_too_big() {
        let (x, y, s) = (2_000_000u128*WAD, 2_000_000u128*WAD, 1_000_000u128*WAD);
        let err = remove_liquidity(x, y, s + 1, s).unwrap_err();
        assert_eq!(err, AmmError::Overflow);
    }

    #[test]
    fn t_remove_liquidity_zero_burn() {
        let err = remove_liquidity(1_000_000u128*WAD, 1_000_000u128*WAD, 0, 1_000_000u128*WAD).unwrap_err();
        assert_eq!(err, AmmError::ZeroAmount);
    }

    #[test]
    fn t_remove_liquidity_min_reserve_guard() {
        let (x, y, s) = (MIN_RESERVE + 10, MIN_RESERVE + 10, 1_000_000u128*WAD);
        let err = remove_liquidity(x, y, 999_999u128*WAD, s).unwrap_err();
        assert_eq!(err, AmmError::MinReserveBreached);
    }
}
