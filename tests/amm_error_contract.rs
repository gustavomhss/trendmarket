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
                error_code: "CE_AMM_0001",
                user_message: "amount deve ser > 0",
                http_status: 422,
            },
        ),
        AmmError::ZeroReserve => (
            "ZeroReserve",
            RustCatalogEntry {
                error_code: "CE_AMM_0002",
                user_message: "reserve deve ser > 0",
                http_status: 422,
            },
        ),
        AmmError::MinReserveBreached => (
            "MinReserveBreached",
            RustCatalogEntry {
                error_code: "CE_AMM_0003",
                user_message: "reserva ficaria abaixo do mínimo",
                http_status: 422,
            },
        ),
        AmmError::Overflow => (
            "Overflow",
            RustCatalogEntry {
                error_code: "CE_AMM_0004",
                user_message: "overflow/underflow numérico",
                http_status: 500,
            },
        ),
        AmmError::InputTooSmall => (
            "InputTooSmall",
            RustCatalogEntry {
                error_code: "CE_AMM_0005",
                user_message: "input efetivo após taxa é 0",
                http_status: 422,
            },
        ),
        AmmError::InvalidFee => (
            "InvalidFee",
            RustCatalogEntry {
                error_code: "CE_AMM_0006",
                user_message: "fee_ppm deve ser ≤ 1e6",
                http_status: 422,
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

/// Parser YAML minimalista suficiente para o formato `variant -> {error_code, ...}`.
/// Usamos esse fallback para não introduzir dependências extras no caminho de build
/// (o ambiente de CI offline já distribui o catálogo como artefato versionado).
fn parse_catalog(contents: &str) -> BTreeMap<String, CatalogEntry> {
    let mut catalog = BTreeMap::new();
    let mut current_variant: Option<String> = None;

    for (idx, raw_line) in contents.lines().enumerate() {
        let line_number = idx + 1;
        let line = raw_line.trim_end();
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }

        if !line.starts_with(' ') {
            let variant = line
                .strip_suffix(':')
                .unwrap_or_else(|| {
                    panic!("linha {line_number}: esperado ':' após o nome do variant")
                })
                .trim();
            if variant.is_empty() {
                panic!("linha {line_number}: nome de variant vazio");
            }

            let entry = CatalogEntry {
                error_code: String::new(),
                user_message: String::new(),
                http_status: 0,
            };

            let previous = catalog.insert(variant.to_string(), entry);
            if previous.is_some() {
                panic!("linha {line_number}: variant duplicado '{variant}'");
            }

            current_variant = Some(variant.to_string());
            continue;
        }

        let variant = current_variant.as_ref().unwrap_or_else(|| {
            panic!("linha {line_number}: encontrado atributo sem variant associado")
        });
        let trimmed = line.trim();
        let (key, value) = trimmed
            .split_once(':')
            .unwrap_or_else(|| panic!("linha {line_number}: esperado par chave:valor"));
        let key = key.trim();
        let mut value = value.trim();
        if value.is_empty() {
            panic!("linha {line_number}: valor vazio para chave '{key}'");
        }

        if let Some(stripped) = value.strip_prefix('"') {
            value = stripped
                .strip_suffix('"')
                .unwrap_or_else(|| panic!("linha {line_number}: string sem aspas de fechamento"));
        }

        let entry = catalog
            .get_mut(variant)
            .expect("variant deve existir após inserção");

        match key {
            "error_code" => {
                if !entry.error_code.is_empty() {
                    panic!("linha {line_number}: error_code duplicado em '{variant}'");
                }
                entry.error_code = value.to_string();
            }
            "user_message" => {
                if !entry.user_message.is_empty() {
                    panic!("linha {line_number}: user_message duplicado em '{variant}'");
                }
                entry.user_message = value.to_string();
            }
            "http_status" => {
                if entry.http_status != 0 {
                    panic!("linha {line_number}: http_status duplicado em '{variant}'");
                }
                entry.http_status = value
                    .parse::<u16>()
                    .unwrap_or_else(|_| panic!("linha {line_number}: http_status inválido"));
            }
            other => panic!("linha {line_number}: chave desconhecida '{other}'"),
        }
    }

    for (variant, entry) in &catalog {
        if entry.error_code.is_empty() || entry.user_message.is_empty() || entry.http_status == 0 {
            panic!("variant '{variant}' incompleto no catálogo");
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
