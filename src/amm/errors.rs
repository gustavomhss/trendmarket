//! Erros padronizados do AMM
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmmError {
    ZeroAmount,
    ZeroReserve,
    MinReserveBreached,
    Overflow,
    InputTooSmall,
}

impl fmt::Display for AmmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use AmmError::*;
        let s = match self {
            ZeroAmount => "amount deve ser > 0",
            ZeroReserve => "reserve deve ser > 0",
            MinReserveBreached => "reserva ficaria abaixo do mínimo",
            Overflow => "overflow/underflow numérico",
            InputTooSmall => "input efetivo após taxa é 0",
        };
        write!(f, "{}", s)
    }
}

impl std::error::Error for AmmError {}
