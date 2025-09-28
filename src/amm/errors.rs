//! Erros padronizados do AMM
use core::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmmError {
    ZeroAmount,
    ZeroReserve,
    MinReserveBreached,
    Overflow,
    InputTooSmall,
    InvalidFee,
}

impl AmmError {
    #[inline]
    pub fn error_code(&self) -> &'static str {
        use AmmError::*;
        match self {
            ZeroAmount => "CE-AMM-0001",
            ZeroReserve => "CE-AMM-0002",
            MinReserveBreached => "CE-AMM-0003",
            Overflow => "CE-AMM-0004",
            InputTooSmall => "CE-AMM-0005",
            InvalidFee => "CE-AMM-0006",
        }
    }

    #[inline]
    pub fn user_message(&self) -> &'static str {
        use AmmError::*;
        match self {
            ZeroAmount => "Swap amount must be greater than zero.",
            ZeroReserve => "Pool reserves must be greater than zero.",
            MinReserveBreached => "Operation would breach the minimum reserve requirement.",
            Overflow => "Numerical overflow or underflow detected while processing the request.",
            InputTooSmall => "Effective input after fees is zero; increase the provided amount.",
            InvalidFee => "Fee in parts-per-million must be less than or equal to 1,000,000.",
        }
    }

    #[inline]
    pub fn http_status(&self) -> Option<u16> {
        use AmmError::*;
        match self {
            ZeroAmount => Some(422),
            ZeroReserve => Some(500),
            MinReserveBreached => Some(409),
            Overflow => Some(500),
            InputTooSmall => Some(422),
            InvalidFee => Some(422),
        }
    }
}

impl fmt::Display for AmmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for AmmError {}
