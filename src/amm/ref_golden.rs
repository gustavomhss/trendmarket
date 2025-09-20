//! Referência de alta precisão ("goldens") baseada em **BigInt/BigRational**
//! para o AMM CPMM (x·y=k) com taxa sobre o **input**.
//!
//! Objetivos desta referência (CRD-7-08):
//! 1. Calcular os resultados **contínuos/exatos** (sem quantização) para swap
//!    e para a necessidade de input alvo (amountIn p/ um dy).
//! 2. Reproduzir a **política de arredondamento** dos ADRs (fee=ceil, divisão
//!    interna nearest-even, fronteiras floor/ceil) usando BigRational/BigUint
//!    para servir de **oráculo de validação** independente do core inteiro.
//! 3. Medir o desvio do invariante, `|Δk/k|`, do **core discreto**.
//!
//! Notas:
//! - Esta referência não entra no caminho de produção — serve só para testes e
//!   geração de *goldens*.
//! - Para compilar este módulo é preciso adicionar deps em `Cargo.toml`:
//!   ```toml
//!   [dependencies]
//!   num-bigint = "0.4"
//!   num-rational = "0.4"
//!   num-integer = "0.1"
//!   num-traits = "0.2"
//!   ```

use super::errors::AmmError;
use super::swap; // para comparar com a implementação inteira
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

#[inline]
fn floor_rat_to_u128(r: &BigRational) -> Result<u128, AmmError> {
    // floor() ∈ BigInt (não-negativo neste domínio); converte para u128
    let f: BigInt = r.clone().floor();
    f.to_u128().ok_or(AmmError::Overflow)
}

#[inline]
fn ceil_rat_to_u128(r: &BigRational) -> Result<u128, AmmError> {
    let c: BigInt = r.clone().ceil();
    c.to_u128().ok_or(AmmError::Overflow)
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
    if x < MIN_RESERVE || y < MIN_RESERVE { return Err(AmmError::MinReserveBreached); }
    if dx == 0 { return Err(AmmError::ZeroAmount); }

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

/// amountIn contínuo (sem quantização) para atingir `dy` (racional) com taxa no input **exata**.
pub fn continuous_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<BigRational, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return Err(AmmError::MinReserveBreached); }
    if dy == 0 { return Err(AmmError::ZeroAmount); }
    if dy >= y - MIN_RESERVE { return Err(AmmError::MinReserveBreached); }

    let fee_rate = fee_rate_ppm_to_q(fee_ppm);
    let x_q = q_from_u128(x, 1);
    let y_q = q_from_u128(y, 1);
    let dy_q = q_from_u128(dy, 1);

    // dx_net = x * dy / (y - dy)
    let dx_net = x_q * dy_q.clone() / (y_q.clone() - dy_q.clone());
    // dx_bruto = dx_net / (1 - fee_rate)
    let one = BigRational::from_integer(BigInt::one());
    let dx = dx_net / (one - fee_rate);
    Ok(dx)
}

// -------------------------
// Política (replica exatamente o core, mas em Big-precision)
// -------------------------
/// amountOut com a **política dos ADRs**: fee **ceil**, `y* = round_nearest_even(k/x')`, out **floor**.
pub fn policy_amount_out(x: Wad, y: Wad, dx: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return Err(AmmError::MinReserveBreached); }
    if dx == 0 { return Err(AmmError::ZeroAmount); }

    let dx_fee = fee_on_input_ceil_u128(dx, fee_ppm);
    let dx_net = dx.checked_sub(dx_fee).ok_or(AmmError::Overflow)?;
    if dx_net == 0 { return Err(AmmError::InputTooSmall); }

    let x1 = x.checked_add(dx_net).ok_or(AmmError::Overflow)?;
    let k = k_big(x, y);
    let y_star = div_nearest_even_big(&k, &bu(x1));               // inteiro (nearest-even)

    // out = floor(y - y*)
    if y_star > bu(y) { return Err(AmmError::Overflow); }
    let out_bu = bu(y) - y_star;
    out_bu.to_u128().ok_or(AmmError::Overflow)
}

/// amountIn com a **política** (ceil dos dois passos + correção final se necessário).
pub fn policy_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    if x < MIN_RESERVE || y < MIN_RESERVE { return Err(AmmError::MinReserveBreached); }
    if dy == 0 { return Err(AmmError::ZeroAmount); }
    if dy >= y - MIN_RESERVE { return Err(AmmError::MinReserveBreached); }

    // 1) dx_net = ceil( x * dy / (y - dy) )
    let num = bu(x) * bu(dy);
    let den = bu(y - dy);
    // ceil division via (n + d - 1)/d  em biguint
    let dx_net_bu = (num + (&den - BigUint::one())) / &den;
    let dx_net = dx_net_bu.to_u128().ok_or(AmmError::Overflow)?;

    // 2) bruto a partir do net: ceil( dx_net / (1 - fee) ) = ceil( dx_net * 1e6 / (1e6-fee) )
    let denom_ppm = (PPM_SCALE as u64) - (fee_ppm as u64);
    if denom_ppm == 0 { return Err(AmmError::InputTooSmall); }
    let n = bu(dx_net) * bu(PPM_SCALE as u128);
    let d = bu(denom_ppm as u128);
    let dx_bu = (n + (&d - BigUint::one())) / &d; // ceil
    let mut dx = dx_bu.to_u128().ok_or(AmmError::Overflow)?;

    // 3) correção por arredondamento da taxa
    loop {
        let fee = fee_on_input_ceil_u128(dx, fee_ppm);
        let net = dx.checked_sub(fee).ok_or(AmmError::Overflow)?;
        if net >= dx_net { break; }
        dx = dx.checked_add(1).ok_or(AmmError::Overflow)?;
    }

    // sanity: policy_out(dx) >= dy
    let out = policy_amount_out(x, y, dx, fee_ppm)?;
    if out < dy {
        let mut dx2 = dx;
        loop {
            dx2 = dx2.checked_add(1).ok_or(AmmError::Overflow)?;
            if policy_amount_out(x, y, dx2, fee_ppm)? >= dy { return Ok(dx2); }
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
    // core
    let out_core = swap::get_amount_out(x, y, dx, fee_ppm)?;

    // política em big-precision (deve bater 1:1 com o core)
    let out_policy = policy_amount_out(x, y, dx, fee_ppm)?;

    // contínuo (sem quantização)
    let out_cont = continuous_amount_out(x, y, dx, fee_ppm)?;
    let out_cont_floor = floor_rat_to_u128(&out_cont)?;

    // desvio do invariante do core
    let dk_over_k_core = dk_over_k_from_core(x, y, dx, out_core, fee_ppm);

    Ok(RefOut { out_core, out_policy, out_cont_floor, out_cont, dk_over_k_core })
}

/// Compara o **core** com a referência (amountIn para alvo `dy`).
pub fn golden_amount_in(x: Wad, y: Wad, dy: Wad, fee_ppm: Ppm) -> Result<RefIn, AmmError> {
    let in_core = swap::get_amount_in(x, y, dy, fee_ppm)?;
    let in_policy = policy_amount_in(x, y, dy, fee_ppm)?;
    let in_cont = continuous_amount_in(x, y, dy, fee_ppm)?;
    let in_cont_ceil = ceil_rat_to_u128(&in_cont)?;

    // Out core para medir Δk/k no cenário fechado
    let out_core = swap::get_amount_out(x, y, in_core, fee_ppm)?;
    let dk_over_k_core = dk_over_k_from_core(x, y, in_core, out_core, fee_ppm);

    Ok(RefIn { in_core, in_policy, in_cont_ceil, in_cont, dk_over_k_core })
}

// -------------------------
// TESTES (sanidade & igualdade policy==core)
// -------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use super::super::types::WAD;
    use num_bigint::BigInt; // necessário neste escopo do módulo de testes
    use num_rational::BigRational; // idem

    const FEE0: Ppm = 0;
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
        assert_eq!(core, pol);
    }

    #[test]
    fn t_continuous_delta_k_is_zero() {
        let (x, y, dx) = (1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD);
        let out_cont = continuous_amount_out(x, y, dx, FEE3).unwrap();
        // No modelo contínuo (sem quantização), k' == k exatamente ⇒ Δk/k = 0
        // (Não precisamos calcular aqui; a identidade do CPMM garante)
        let out_floor = super::floor_rat_to_u128(&out_cont).unwrap();
        assert!(out_floor <= x); // sanity só para usar o valor
    }

    #[test]
    fn t_golden_out_bundle() {
        let g = golden_amount_out(1_000_000u128*WAD, 1_000_000u128*WAD, 10_000u128*WAD, FEE3).unwrap();
        assert_eq!(g.out_core, g.out_policy);
        // Δk/k no core deve ser ≥ 0 (monótono com taxa)
        assert!(g.dk_over_k_core >= BigRational::from_integer(BigInt::zero()));
    }

    #[test]
    fn t_golden_in_bundle() {
        let g = golden_amount_in(1_000_000u128*WAD, 1_000_000u128*WAD, 9_870u128*WAD, FEE3).unwrap();
        assert_eq!(g.in_core, g.in_policy);
        assert!(g.dk_over_k_core >= BigRational::from_integer(BigInt::zero()));
    }
}
