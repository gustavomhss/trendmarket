use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use credit_engine_core::amm::errors::AmmError;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogEntry {
    error_code: String,
    user_message: String,
    http_status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RustCatalogEntry {
    error_code: &'static str,
    user_message: &'static str,
    http_status: u16,
}

fn entry_for(error: AmmError) -> (&'static str, RustCatalogEntry) {
    match error {
        AmmError::ZeroAmount => (
            "ZeroAmount",
            RustCatalogEntry {
                error_code: "CE-AMM-0001",
                user_message: "Input amount must be greater than zero.",
                http_status: 400,
            },
        ),
        AmmError::ZeroReserve => (
            "ZeroReserve",
            RustCatalogEntry {
                error_code: "CE-AMM-0002",
                user_message: "Reserves must stay above zero.",
                http_status: 400,
            },
        ),
        AmmError::MinReserveBreached => (
            "MinReserveBreached",
            RustCatalogEntry {
                error_code: "CE-AMM-0003",
                user_message: "Operation would breach the minimum reserve.",
                http_status: 409,
            },
        ),
        AmmError::Overflow => (
            "Overflow",
            RustCatalogEntry {
                error_code: "CE-AMM-0004",
                user_message: "Numerical overflow or underflow detected.",
                http_status: 500,
            },
        ),
        AmmError::InputTooSmall => (
            "InputTooSmall",
            RustCatalogEntry {
                error_code: "CE-AMM-0005",
                user_message: "Effective input amount is too small.",
                http_status: 422,
            },
        ),
        AmmError::InvalidFee => (
            "InvalidFee",
            RustCatalogEntry {
                error_code: "CE-AMM-0006",
                user_message: "Fee ppm must be at most 1,000,000.",
                http_status: 400,
            },
        ),
    }
}

fn rust_catalog() -> BTreeMap<String, RustCatalogEntry> {
    use AmmError::*;

    [
        ZeroAmount,
        ZeroReserve,
        MinReserveBreached,
        Overflow,
        InputTooSmall,
        InvalidFee,
    ]
    .into_iter()
    .map(entry_for)
    .map(|(name, entry)| (name.to_string(), entry))
    .collect()
}

fn load_catalog_from_yaml() -> BTreeMap<String, CatalogEntry> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ops/errors/catalog_amm.yaml");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("não foi possível ler {path:?}: {err}"));

    parse_catalog(&content)
}

/// Parser YAML minimalista suficiente para o formato {variant, code, default_message, http_status}.
fn parse_catalog(contents: &str) -> BTreeMap<String, CatalogEntry> {
    use serde::Deserialize;

    #[derive(Debug, Deserialize)]
    struct Root {
        errors: Vec<Inner>,
    }

    #[derive(Debug, Deserialize)]
    struct Inner {
        variant: String,
        code: String,
        default_message: String,
        http_status: u16,
    }

    let parsed: Root =
        serde_yaml::from_str(contents).expect("falha ao parsear ops/errors/catalog_amm.yaml");

    let mut catalog = BTreeMap::new();
    for entry in parsed.errors {
        let previous = catalog.insert(
            entry.variant.clone(),
            CatalogEntry {
                error_code: entry.code,
                user_message: entry.default_message,
                http_status: entry.http_status,
            },
        );
        if previous.is_some() {
            panic!("variant duplicado no catálogo YAML: {}", entry.variant);
        }
    }

    catalog
}

#[test]
fn amm_error_catalog_matches_yaml() {
    let rust_catalog = rust_catalog();
    let yaml_catalog = load_catalog_from_yaml();

    let rust_variants: BTreeSet<_> = rust_catalog.keys().cloned().collect();
    let yaml_variants: BTreeSet<_> = yaml_catalog.keys().cloned().collect();

    assert_eq!(
        rust_variants, yaml_variants,
        "catálogo YAML e mapeamento Rust divergem nos variants"
    );

    for (variant, rust_entry) in &rust_catalog {
        let yaml_entry = yaml_catalog
            .get(variant)
            .unwrap_or_else(|| panic!("variant {variant} ausente do catálogo YAML"));

        assert_eq!(
            rust_entry.error_code,
            yaml_entry.error_code.as_str(),
            "error_code divergente para variant {variant}"
        );
        assert_eq!(
            rust_entry.user_message,
            yaml_entry.user_message.as_str(),
            "user_message divergente para variant {variant}"
        );
        assert_eq!(
            rust_entry.http_status, yaml_entry.http_status,
            "http_status divergente para variant {variant}"
        );
    }
}
