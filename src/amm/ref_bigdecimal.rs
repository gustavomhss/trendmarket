//! Referência de alta precisão ("goldens") baseada em **BigInt/BigRational**
//! para o AMM CPMM (x·y=k) com taxa sobre o **input**.

use super::errors::AmmError;
use super::swap; // comparar com a implementação inteira
use super::types::{Ppm, Wad, U256, PPM_SCALE, MIN_RESERVE};

use num_bigint::{BigInt, BigUint};
use num_integer::Integer; // div_rem, is_odd/is_even
use num_rational::BigRational;
use num_traits::{One, ToPrimitive, Zero};

// -------------------------
// Helpers de conversão & arredondamento
// -------------------------
#[inline]
fn bu(v: Wad) -> BigUint { BigUint::from(v) }
#[inline]
fn bi_u(v: u128) -> BigInt { BigInt::from(v) }
#[inline]
fn bu_to_bi(v: &BigUint) -> BigInt { BigInt::from(v.clone()) }
#[inline]
fn q_from_u128(n: u128, d: u128) -> BigRational { BigRational::new(bi_u(n), bi_u(d)) }
#[inline]
fn q_from_bu(n: &BigUint, d: &BigUint) -> BigRational { BigRational::new(bu_to_bi(n), bu_to_bi(d)) }

/// Divide `n/d` com **nearest (ties-to-even)** e retorna inteiro BigUint.
fn div_nearest_even_big(n: &BigUint, d: &BigUint) -> BigUint {
    let (q, r) = n.div_rem(d);
    let two_r = &r << 1;
    if two_r < *d { return q; }
    if two_r > *d { return q + BigUint::one(); }
    // empate: arredonda para o par
    if q.is_odd() { q + BigUint::one() } else { q }
}

/// floor(r) para r ≥ 0, retorna u128
#[inline]
fn floor_rat_to_u128(r: &BigRational) -> Result<u128, AmmError> {
    let n = r.numer().clone();
    let d = r.denom().clone();
    let q = n / d; // floor
    q.to_u128().ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))
}

/// ceil(r) para r ≥ 0, retorna u128
#[inline]
fn ceil_rat_to_u128(r: &BigRational) -> Result<u128, AmmError> {
    let n = r.numer().clone();
    let d = r.denom().clone();
    let (q, rem) = n.div_rem(&d);
    let q = if rem.is_zero() { q } else { q + BigInt::from(1u8) };
    q.to_u128().ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))
}

#[inline]
fn fee_rate_ppm_to_q(fee_ppm: Ppm) -> BigRational {
    BigRational::new(BigInt::from(fee_ppm as i64), BigInt::from(PPM_SCALE as i64))
}

#[inline]
fn k_big(x: Wad, y: Wad) -> BigUint { bu(x) * bu(y) }

#[inline]
fn fee_on_input_ceil_u128(dx: Wad, fee_ppm: Ppm) -> Wad {
    if fee_ppm == 0 { return 0; }
    // (dx * fee_ppm + 1e6-1) / 1e6   — usa U256 para evitar overflow
    let n = U256::from(dx) * U256::from(fee_ppm as u64);
    let d = U256::from(PPM_SCALE as u64);
    let num = n + (d - U256::from(1u8));
    (num / d).as_u128()
}

// -------------------------
// Contínuo/exato (sem quantização)
// -------------------------
/// amountOut contínuo (sem quantização), taxa no input **exata** (sem ceil).
pub fn continuous_amount_out(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<BigRational, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::MinReserveBreached); }
    if dx == 0 { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::ZeroAmount); }

    let fee_rate = fee_rate_ppm_to_q(fee_ppm);                    // r ∈ [0,1]
    let dx_q = q_from_u128(dx, 1);
    let dx_fee = dx_q.clone() * fee_rate;                         // sem ceil
    let dx_net = dx_q - dx_fee;                                   // racional

    let x_q = q_from_u128(x, 1);
    let y_q = q_from_u128(y, 1);
    let k = x_q.clone() * y_q.clone();
    let x1 = x_q + dx_net;
    let y_star = k / x1;                                          // racional
    let out = y_q - y_star;                                       // racional
    Ok(out)
}

/// amountIn contínuo (sem quantização) para atingir `dy` com taxa no input **exata**.
pub fn continuous_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<BigRational, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::MinReserveBreached); }
    if dy == 0 { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::ZeroAmount); }
    if dy >= y - MIN_RESERVE { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::MinReserveBreached); }

    let fee_rate = fee_rate_ppm_to_q(fee_ppm);
    let x_q = q_from_u128(x, 1);
    let y_q = q_from_u128(y, 1);
    let dy_q = q_from_u128(dy, 1);

    // dx_net = x * dy / (y - dy)
    let dx_net = x_q * dy_q.clone() / (y_q.clone() - dy_q.clone());
    // dx_bruto = dx_net / (1 - fee_rate)
    let one = BigRational::new(BigInt::from(1), BigInt::from(1));
    let dx = dx_net / (one - fee_rate);
    Ok(dx)
}

// -------------------------
// Política (replica o core em Big-precision)
// -------------------------
/// amountOut com a **política dos ADRs**: fee **ceil**, y* **nearest-even**, out **floor**.
pub fn policy_amount_out(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::MinReserveBreached); }
    if dx == 0 { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::ZeroAmount); }

    let dx_fee = fee_on_input_ceil_u128(dx, fee_ppm);
    let dx_net = dx.checked_sub(dx_fee).ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))?;
    if dx_net == 0 { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::InputTooSmall); }

    let x1 = x.checked_add(dx_net).ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))?;
    let k = k_big(x, y);
    let y_star = div_nearest_even_big(&k, &bu(x1));               // inteiro (nearest-even)

    // out = floor(y - y*)
    if y_star > bu(y) { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric); }
    let out_bu = bu(y) - y_star;
    out_bu.to_u128().ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))
}

/// amountIn com a **política** (ceil dos dois passos + correção final se necessário).
pub fn policy_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::MinReserveBreached); }
    if dy == 0 { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::ZeroAmount); }
    if dy >= y - MIN_RESERVE { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::MinReserveBreached); }

    // 1) dx_net = ceil( x * dy / (y - dy) )
    let num = bu(x) * bu(dy);
    let den = bu(y - dy);
    let dx_net_bu = (num + (&den - BigUint::one())) / &den; // ceil
    let dx_net = dx_net_bu.to_u128().ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))?;

    // 2) bruto a partir do net: ceil( dx_net * 1e6 / (1e6-fee) )
    let denom_ppm = (PPM_SCALE as u64) - (fee_ppm as u64);
    if denom_ppm == 0 { return crate::amm_bail!(crate::amm::error_catalog::AmmErrorCode::InputTooSmall); }
    let n = bu(dx_net) * bu(PPM_SCALE as u128);
    let d = bu(denom_ppm as u128);
    let dx_bu = (n + (&d - BigUint::one())) / &d; // ceil
    let mut dx = dx_bu.to_u128().ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))?;

    // 3) correção por arredondamento da taxa (garantir net >= dx_net)
    loop {
        let fee = fee_on_input_ceil_u128(dx, fee_ppm);
        let net = dx.checked_sub(fee).ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))?;
        if net >= dx_net { break; }
        dx = dx.checked_add(1).ok_or(crate::amm_err!(crate::amm::error_catalog::AmmErrorCode::OverflowNumeric))?;
    }

    // 4) **AJUSTE PARA BAIXO**: garantir que dx é o **mínimo** que ainda entrega dy
    while let Some(prev) = dx.checked_sub(1) {
        if policy_amount_out(x, y, prev, fee_ppm)? >= dy {
            dx = prev; // ainda entrega? então reduz
        } else {
            break;     // parou de entregar — prev é insuficiente
        }
    }

    Ok(dx)
}

// -------------------------
// Estruturas de comparação (goldens)
// -------------------------
#[derive(Debug, Clone)]
pub struct RefOut {
    pub out_core: Wad,
    pub out_policy: Wad,
    pub out_cont_floor: Wad,
    pub out_cont: BigRational,
    pub dk_over_k_core: BigRational,   // |k1_core - k0| / k0
}

#[derive(Debug, Clone)]
pub struct RefIn {
    pub in_core: Wad,
    pub in_policy: Wad,
    pub in_cont_ceil: Wad,
    pub in_cont: BigRational,
    pub dk_over_k_core: BigRational,
}

fn dk_over_k_from_core(x: Wad, y: Wad, dx: Wad, out: Wad, fee_ppm: Ppm) -> BigRational {
    // k0 = x*y ; x1 = x + (dx - fee_ceil) ; y1 = y - out
    let k0 = k_big(x, y);
    let fee = fee_on_input_ceil_u128(dx, fee_ppm);
    let x1 = x + (dx - fee);
    let y1 = y - out;
    let k1 = k_big(x1, y1);
    let num = if k1 >= k0 { k1.clone() - k0.clone() } else { k0.clone() - k1.clone() };
    q_from_bu(&num, &k0)
}

/// Compara o **core** com a referência (swap X→Y).
pub fn golden_amount_out(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<RefOut, AmmError> {
    let out_core = swap::get_amount_out(x, y, dx, fee_ppm)?;
    let out_policy = policy_amount_out(x, y, dx, fee_ppm)?;
    let out_cont = continuous_amount_out(x, y, dx, fee_ppm)?;
    let out_cont_floor = floor_rat_to_u128(&out_cont)?;
    let dk_over_k_core = dk_over_k_from_core(x, y, dx, out_core, fee_ppm);
    Ok(RefOut { out_core, out_policy, out_cont_floor, out_cont, dk_over_k_core })
}

/// Compara o **core** com a referência (amountIn para alvo `dy`).
pub fn golden_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<RefIn, AmmError> {
    let in_core = swap::get_amount_in(x, y, dy, fee_ppm)?;
    let in_policy = policy_amount_in(x, y, dy, fee_ppm)?;
    let in_cont = continuous_amount_in(x, y, dy, fee_ppm)?;
    let in_cont_ceil = ceil_rat_to_u128(&in_cont)?;
    let out_core = swap::get_amount_out(x, y, in_core, fee_ppm)?;
    let dk_over_k_core = dk_over_k_from_core(x, y, in_core, out_core, fee_ppm);
    Ok(RefIn { in_core, in_policy, in_cont_ceil, in_cont, dk_over_k_core })
}

// -------------------------
// TESTES (único bloco)
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::WAD;

    const FEE3: Ppm = 3000; // 0,30%

    #[test]
    fn t_policy_matches_core_out() {
        let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD);
        let core = swap::get_amount_out(x, y, dx, FEE3).unwrap();
        let pol = policy_amount_out(x, y, dx, FEE3).unwrap();
        assert_eq!(core, pol);
    }

    #[test]
    fn t_policy_matches_core_in() {
        let (x, y, dy) = (1_000_000u128*WAD, 1_000_000u128*WAD, 9_870u128*WAD);
        let core = swap::get_amount_in(x, y, dy, FEE3).unwrap();
        let pol = policy_amount_in(x, y, dy, FEE3).unwrap();
        let diff = if core >= pol { core - pol } else { pol - core };
        assert!(diff <= 1, "in_core={} in_policy={} diff={}", core, pol, diff);
    }

    #[test]
    fn t_golden_out_bundle() {
        let g = golden_amount_out(1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD, FEE3).unwrap();
        assert_eq!(g.out_core, g.out_policy);
        assert!(g.dk_over_k_core >= BigRational::from_integer(BigInt::from(0)));
    }

    #[test]
    fn t_golden_in_bundle() {
        let g = golden_amount_in(1_000_000u128*WAD, 1_000_000u128*WAD, 9_870u128*WAD, FEE3).unwrap();
        let diff = if g.in_core >= g.in_policy { g.in_core - g.in_policy } else { g.in_policy - g.in_core };
        assert!(diff <= 1, "in_core={} in_policy={} diff={}", g.in_core, g.in_policy, diff);
        assert!(g.dk_over_k_core >= BigRational::from_integer(BigInt::from(0)));
    }
}
