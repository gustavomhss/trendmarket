//! Compat: reexporta o módulo de erros unificado (A120).

pub use super::error::{AmmError, Result};
pub use super::error_catalog::{AmmErrorCode, default_locale_message};
pub use super::error_map::{from_swap_inputs, to_error};
