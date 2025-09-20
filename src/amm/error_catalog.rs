//! Catálogo imutável de erros do AMM.
use core::fmt;

/// Código de erro do AMM.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum AmmErrorCode {
    /// Operações com montante de entrada zero.
    ZeroAmount,
    /// Reservas não inicializadas ou zeradas.
    ZeroReserve,
    /// Operação violaria a reserva mínima.
    MinReserveBreached,
    /// Overflow ou underflow em cálculos numéricos.
    OverflowNumeric,
    /// Taxa efetiva zera o input líquido.
    EffectiveInputZero,
}

impl AmmErrorCode {
    /// Código textual estável do erro.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ZeroAmount => "AMM-0001",
            Self::ZeroReserve => "AMM-0002",
            Self::MinReserveBreached => "AMM-0003",
            Self::OverflowNumeric => "AMM-0004",
            Self::EffectiveInputZero => "AMM-0005",
        }
    }

    /// Título curto em português.
    pub const fn title(&self) -> &'static str {
        match self {
            Self::ZeroAmount => "Quantidade zerada",
            Self::ZeroReserve => "Reserva zerada",
            Self::MinReserveBreached => "Reserva mínima violada",
            Self::OverflowNumeric => "Overflow numérico",
            Self::EffectiveInputZero => "Input efetivo zerado",
        }
    }

    /// Mensagem base em português.
    pub const fn message_pt(&self) -> &'static str {
        match self {
            Self::ZeroAmount => "amount deve ser > 0",
            Self::ZeroReserve => "reserve deve ser > 0",
            Self::MinReserveBreached => "reserva ficaria abaixo do mínimo",
            Self::OverflowNumeric => "overflow/underflow numérico",
            Self::EffectiveInputZero => "input efetivo após taxa é 0",
        }
    }

    /// Retorna todas as variantes em ordem estável.
    pub fn all() -> &'static [AmmErrorCode] {
        const ALL: &[AmmErrorCode] = &[
            AmmErrorCode::ZeroAmount,
            AmmErrorCode::ZeroReserve,
            AmmErrorCode::MinReserveBreached,
            AmmErrorCode::OverflowNumeric,
            AmmErrorCode::EffectiveInputZero,
        ];
        ALL
    }
}

impl fmt::Display for AmmErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

/// Mensagem padrão na localidade ativa (pt-BR).
pub fn default_locale_message(code: AmmErrorCode) -> &'static str {
    code.message_pt()
}