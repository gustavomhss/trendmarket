use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use credit_engine_core::amm::errors::{AmmError, AmmErrorDescriptor};

const ALLOWED_HTTP_STATUSES: &[u16] = &[400, 403, 404, 409, 500, 502, 503];
const CODE_PREFIX: &str = "CE-AMM-";
const CODE_DIGITS: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogEntry {
    error_code: String,
    user_message: String,
    http_status: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CatalogMeta {
    domain: String,
    prefix: String,
    version: u32,
}

fn load_catalog_from_yaml() -> (CatalogMeta, BTreeMap<String, CatalogEntry>) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ops/errors/catalog_amm.yaml");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("não foi possível ler {path:?}: {err}"));

    parse_catalog(&content)
}

fn parse_catalog(contents: &str) -> (CatalogMeta, BTreeMap<String, CatalogEntry>) {
    fn trim_quotes(input: &str) -> String {
        let trimmed = input.trim();
        if let Some(stripped) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
            stripped.to_string()
        } else {
            trimmed.to_string()
        }
    }

    fn split_inline_map(body: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for ch in body.chars() {
            match ch {
                '"' => {
                    in_quotes = !in_quotes;
                    current.push(ch);
                }
                ',' if !in_quotes => {
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                    }
                    current.clear();
                }
                _ => current.push(ch),
            }
        }

        if !current.trim().is_empty() {
            parts.push(current.trim().to_string());
        }

        parts
    }

    fn parse_inline_map(body: &str) -> BTreeMap<String, String> {
        let mut map = BTreeMap::new();
        for entry in split_inline_map(body) {
            let mut segments = entry.splitn(2, ':');
            let key = segments
                .next()
                .unwrap_or_else(|| panic!("entrada inválida: {entry}"))
                .trim();
            let value = segments
                .next()
                .unwrap_or_else(|| panic!("entrada inválida: {entry}"));
            map.insert(key.to_string(), trim_quotes(value));
        }
        map
    }

    let mut meta: Option<CatalogMeta> = None;
    let mut catalog = BTreeMap::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with("meta:") {
            let start = trimmed
                .find('{')
                .unwrap_or_else(|| panic!("linha meta inválida: {trimmed}"));
            let end = trimmed
                .rfind('}')
                .unwrap_or_else(|| panic!("linha meta inválida: {trimmed}"));
            let map = parse_inline_map(&trimmed[start + 1..end]);
            meta = Some(CatalogMeta {
                domain: map
                    .get("domain")
                    .cloned()
                    .unwrap_or_else(|| panic!("meta domain ausente")),
                prefix: map
                    .get("prefix")
                    .cloned()
                    .unwrap_or_else(|| panic!("meta prefix ausente")),
                version: map
                    .get("version")
                    .unwrap_or_else(|| panic!("meta version ausente"))
                    .parse()
                    .unwrap_or_else(|err| panic!("meta version inválida: {err}")),
            });
            continue;
        }

        if trimmed.starts_with('-') {
            let body_start = trimmed
                .find('{')
                .unwrap_or_else(|| panic!("linha de erro inválida: {trimmed}"))
                + 1;
            let body_end = trimmed
                .rfind('}')
                .unwrap_or_else(|| panic!("linha de erro inválida: {trimmed}"));
            let map = parse_inline_map(&trimmed[body_start..body_end]);

            let variant = map
                .get("variant")
                .cloned()
                .unwrap_or_else(|| panic!("variant ausente em {trimmed}"));
            let code = map
                .get("code")
                .cloned()
                .unwrap_or_else(|| panic!("code ausente em {trimmed}"));
            let message = map
                .get("default_message")
                .cloned()
                .unwrap_or_else(|| panic!("default_message ausente em {trimmed}"));
            let http_status = map
                .get("http_status")
                .unwrap_or_else(|| panic!("http_status ausente em {trimmed}"))
                .parse()
                .unwrap_or_else(|err| panic!("http_status inválido: {err}"));

            let previous = catalog.insert(
                variant.clone(),
                CatalogEntry {
                    error_code: code,
                    user_message: message,
                    http_status,
                },
            );
            if previous.is_some() {
                panic!("variant duplicado no catálogo YAML: {}", variant);
            }
        }
    }

    let meta = meta.expect("seção meta ausente no catálogo YAML");
    (meta, catalog)
}

fn descriptors_map(descriptors: &[AmmErrorDescriptor]) -> BTreeMap<String, AmmErrorDescriptor> {
    descriptors
        .iter()
        .map(|descriptor| (descriptor.variant.variant_name().to_string(), *descriptor))
        .collect()
}

#[test]
fn amm_error_catalog_matches_yaml() {
    let descriptors = AmmError::descriptors();
    assert_eq!(descriptors.len(), AmmError::ALL_VARIANTS.len());

    let (yaml_meta, yaml_catalog) = load_catalog_from_yaml();

    assert_eq!(yaml_meta.domain, "AMM", "domínio YAML inválido");
    assert_eq!(yaml_meta.prefix, "CE-AMM", "prefixo YAML inválido");
    assert_eq!(yaml_meta.version, 1, "versão YAML inesperada");

    let rust_catalog = descriptors_map(descriptors);

    let rust_variants: BTreeSet<_> = rust_catalog.keys().cloned().collect();
    let yaml_variants: BTreeSet<_> = yaml_catalog.keys().cloned().collect();

    assert_eq!(
        rust_variants, yaml_variants,
        "catálogo YAML e mapeamento Rust divergem nos variants"
    );

    let mut seen_codes = BTreeSet::new();

    for (variant, descriptor) in &rust_catalog {
        let yaml_entry = yaml_catalog
            .get(variant)
            .unwrap_or_else(|| panic!("variant {variant} ausente do catálogo YAML"));

        let code = descriptor.code;
        assert!(
            code.starts_with(CODE_PREFIX),
            "error_code de {variant} deve começar com {CODE_PREFIX}"
        );
        let suffix = &code[CODE_PREFIX.len()..];
        assert_eq!(
            suffix.len(),
            CODE_DIGITS,
            "error_code de {variant} deve ter {CODE_DIGITS} dígitos"
        );
        assert!(
            suffix.chars().all(|c| c.is_ascii_digit()),
            "error_code de {variant} deve conter apenas dígitos"
        );
        assert!(
            seen_codes.insert(code),
            "error_code duplicado detectado: {code}"
        );

        let message = descriptor.message.trim();
        assert!(!message.is_empty(), "user_message de {variant} está vazio");
        assert!(
            message.ends_with('.'),
            "user_message de {variant} deve terminar com ponto"
        );

        assert!(
            ALLOWED_HTTP_STATUSES.contains(&descriptor.http_status),
            "http_status inválido ({}) para {variant}",
            descriptor.http_status
        );

        assert_eq!(
            descriptor.variant.error_code(),
            code,
            "error_code() divergente para {variant}"
        );
        assert_eq!(
            descriptor.variant.user_message(),
            descriptor.message,
            "user_message() divergente para {variant}"
        );
        assert_eq!(
            descriptor.variant.http_status(),
            descriptor.http_status,
            "http_status() divergente para {variant}"
        );

        assert_eq!(
            descriptor.code,
            yaml_entry.error_code.as_str(),
            "error_code divergente para {variant}"
        );
        assert_eq!(
            descriptor.message,
            yaml_entry.user_message.as_str(),
            "user_message divergente para {variant}"
        );
        assert_eq!(
            descriptor.http_status, yaml_entry.http_status,
            "http_status divergente para {variant}"
        );
    }
}
