pub mod errors; // Shim de compat: reexports da API unificada
pub mod guardrails; // CRD-7-03
pub mod liquidity; // CRD-7-05
pub mod pricing;
pub mod swap; // CRD-7-04
pub mod types; // CRD-7-03 // CRD-7-06

// A120 — módulos unificados de erro
pub mod error;
pub mod error_catalog;
pub mod error_map;
