//! Validações e helpers numéricos seguros para o AMM.
//! Objetivo: entradas seguras e divisões/multiplicações sem estouro.

use super::errors::AmmError;
use super::types::{Wad, MIN_RESERVE, U256};

#[inline]
pub fn ensure_nonzero(amount: Wad) -> Result<(), AmmError> {
    if amount == 0 {
        Err(AmmError::ZeroAmount)
    } else {
        Ok(())
    }
}

#[inline]
pub fn ensure_reserves(x: Wad, y: Wad) -> Result<(), AmmError> {
    if x == 0 || y == 0 {
        return Err(AmmError::ZeroReserve);
    }
    if x < MIN_RESERVE || y < MIN_RESERVE {
        return Err(AmmError::MinReserveBreached);
    }
    Ok(())
}

#[inline]
pub fn checked_add(a: Wad, b: Wad) -> Result<Wad, AmmError> {
    a.checked_add(b).ok_or(AmmError::Overflow)
}

#[inline]
pub fn checked_sub(a: Wad, b: Wad) -> Result<Wad, AmmError> {
    a.checked_sub(b).ok_or(AmmError::Overflow)
}

#[inline]
pub fn mul_u128_to_u256(a: Wad, b: Wad) -> U256 {
    U256::from(a) * U256::from(b)
}

#[inline]
pub fn u256_to_u128_checked(v: U256) -> Result<Wad, AmmError> {
    if v > U256::from(u128::MAX) {
        Err(AmmError::Overflow)
    } else {
        Ok(v.as_u128())
    }
}

/// Divisão com arredondamento *nearest (ties-to-even)* em U256 → U256
pub fn div_nearest_even_u256(n: U256, d: U256) -> Result<U256, AmmError> {
    if d.is_zero() {
        return Err(AmmError::Overflow);
    }
    let q = n / d; // quociente
    let r = n % d; // resto
    let two_r = r << 1; // 2*r
    if two_r < d {
        return Ok(q);
    }
    if two_r > d {
        return Ok(q + U256::from(1u8));
    }
    // empate: arredonda para o par
    if (q & U256::from(1u8)) == U256::from(1u8) {
        Ok(q + U256::from(1u8))
    } else {
        Ok(q)
    }
}

/// Versão que retorna u128 (com checagem de overflow no downcast)
pub fn div_nearest_even_u256_to_u128(n: U256, d: U256) -> Result<Wad, AmmError> {
    let q = div_nearest_even_u256(n, d)?;
    u256_to_u128_checked(q)
}

// -------------------------
// TESTES
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn t_ensure_nonzero() {
        assert!(ensure_nonzero(1).is_ok());
        assert_eq!(ensure_nonzero(0).unwrap_err(), AmmError::ZeroAmount);
    }

    #[test]
    fn t_ensure_reserves() {
        // feliz
        assert!(ensure_reserves(MIN_RESERVE, MIN_RESERVE).is_ok());
        // zero
        assert_eq!(
            ensure_reserves(0, MIN_RESERVE).unwrap_err(),
            AmmError::ZeroReserve
        );
        assert_eq!(
            ensure_reserves(MIN_RESERVE, 0).unwrap_err(),
            AmmError::ZeroReserve
        );
        // abaixo do mínimo
        assert_eq!(
            ensure_reserves(MIN_RESERVE - 1, MIN_RESERVE).unwrap_err(),
            AmmError::MinReserveBreached
        );
        assert_eq!(
            ensure_reserves(MIN_RESERVE, MIN_RESERVE - 1).unwrap_err(),
            AmmError::MinReserveBreached
        );
    }

    #[test]
    fn t_checked_add_sub_over_under_flow() {
        use core::u128::MAX as UMAX;
        // add ok
        assert_eq!(checked_add(1, 2).unwrap(), 3);
        // add overflow
        assert_eq!(checked_add(UMAX, 1).unwrap_err(), AmmError::Overflow);
        // sub ok
        assert_eq!(checked_sub(5, 3).unwrap(), 2);
        // sub underflow
        assert_eq!(checked_sub(0, 1).unwrap_err(), AmmError::Overflow);
    }

    #[test]
    fn t_u256_div_nearest_even_rounding() {
        let two = U256::from(2u8);
        let three = U256::from(3u8);
        let five = U256::from(5u8);

        // 5/2 = 2.5 -> empata, 2 é par -> fica 2
        let q = div_nearest_even_u256(five, two).unwrap();
        assert_eq!(q, U256::from(2u8));
        // 3/2 = 1.5 -> empata, 1 é ímpar -> sobe para 2
        let q = div_nearest_even_u256(three, two).unwrap();
        assert_eq!(q, U256::from(2u8));
    }
}
