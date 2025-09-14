//! Tipos básicos do AMM (escala fixa) + U256 para intermediários.
//! Depende do ADR-0001.

use uint::construct_uint;
construct_uint! {
    /// Inteiro de 256 bits para contas intermediárias seguras.
    pub struct U256(4);
}

pub type Wad = u128;   // escala 1e18
pub type Ppm = u32;    // 0..=1_000_000

pub const WAD: Wad = 1_000_000_000_000_000_000u128; // 1e18
pub const PPM_SCALE: Ppm = 1_000_000;                // 1e6 (ppm)
pub const MIN_RESERVE: Wad = WAD;                    // 1 unidade inteira (1e-18 do ativo)

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reserves { pub x: Wad, pub y: Wad }
impl Reserves {
    pub fn new(x: Wad, y: Wad) -> Self { Self { x, y } }
}
