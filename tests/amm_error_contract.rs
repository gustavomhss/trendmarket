use std::collections::{BTreeMap, BTreeSet};

use credit_engine_core::amm::errors::{AmmError, AmmErrorDescriptor, AMM_ERROR_DESCRIPTORS};
use once_cell::sync::Lazy;
use regex::Regex;

#[path = "catalog_utils.rs"]
mod catalog_utils;

static CODE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^CE-AMM-\d{4}$").unwrap());
const ALLOWED_HTTP_STATUSES: &[u16] = &[400, 403, 404, 409, 500, 502, 503];

fn descriptors_by_variant() -> BTreeMap<String, &'static AmmErrorDescriptor> {
    AMM_ERROR_DESCRIPTORS
        .iter()
        .map(|descriptor| (descriptor.variant.variant_name().to_string(), descriptor))
        .collect()
}

#[test]
fn amm_error_runtime_metadata_is_exhaustive() {
    let catalog = catalog_utils::load_catalog();

    assert_eq!(catalog.meta.domain, "AMM", "domínio inválido no catálogo");
    assert_eq!(
        catalog.meta.prefix, "CE-AMM",
        "prefixo inválido no catálogo"
    );
    assert_eq!(catalog.meta.version, 1, "versão inválida no catálogo");

    let yaml_map: BTreeMap<_, _> = catalog
        .errors
        .iter()
        .map(|entry| (entry.variant.clone(), entry.clone()))
        .collect();
    assert_eq!(
        yaml_map.len(),
        catalog.errors.len(),
        "variants duplicados no catálogo"
    );

    let descriptors = descriptors_by_variant();
    assert_eq!(descriptors.len(), AMM_ERROR_DESCRIPTORS.len());
    assert_eq!(
        descriptors.len(),
        AmmError::ALL_VARIANTS.len(),
        "descriptors e ALL_VARIANTS divergem"
    );

    let mut runtime_variants = BTreeMap::new();
    for variant in AmmError::ALL_VARIANTS {
        let variant_name = variant.variant_name().to_string();
        let code = variant.error_code();
        let message = variant.user_message();
        let status = variant.http_status();

        assert!(
            CODE_REGEX.is_match(code),
            "código inválido {code} para variant {variant_name}"
        );
        assert!(
            ALLOWED_HTTP_STATUSES.contains(&status),
            "http_status {status} fora do contrato para variant {variant_name}"
        );
        assert!(
            !message.trim().is_empty(),
            "mensagem vazia para variant {variant_name}"
        );
        assert_eq!(
            message.trim(),
            message,
            "mensagem contém espaços supérfluos para variant {variant_name}"
        );
        assert!(
            message.ends_with('.'),
            "mensagem deve terminar com ponto para variant {variant_name}"
        );
        assert!(
            !message.contains('\n'),
            "mensagem deve ser single-line para variant {variant_name}"
        );

        runtime_variants.insert(variant_name, (code, message, status));
    }

    assert_eq!(
        runtime_variants.len(),
        AmmError::ALL_VARIANTS.len(),
        "coleta de variantes em tempo de execução está divergente de ALL_VARIANTS",
    );

    let yaml_variants: BTreeSet<_> = yaml_map.keys().cloned().collect();
    let rust_variants: BTreeSet<_> = runtime_variants.keys().cloned().collect();

    assert_eq!(
        rust_variants, yaml_variants,
        "catálogo YAML e enum AmmError não estão em sincronia"
    );

    for (variant_name, (code, message, status)) in runtime_variants {
        let entry = yaml_map
            .get(&variant_name)
            .unwrap_or_else(|| panic!("variant {variant_name} ausente do catálogo"));

        let descriptor = descriptors
            .get(&variant_name)
            .unwrap_or_else(|| panic!("descriptor ausente para {variant_name}"));

        assert_eq!(
            entry.code, code,
            "catálogo divergente para variant {variant_name}"
        );
        assert_eq!(
            entry.message, message,
            "mensagem divergente para variant {variant_name}"
        );
        assert_eq!(
            entry.http_status, status,
            "http_status divergente para variant {variant_name}"
        );

        assert_eq!(
            descriptor.code, code,
            "descriptor divergente (code) para {variant_name}"
        );
        assert_eq!(
            descriptor.message, message,
            "descriptor divergente (message) para {variant_name}"
        );
        assert_eq!(
            descriptor.http_status, status,
            "descriptor divergente (http_status) para {variant_name}"
        );
    }
}

#[test]
fn catalog_does_not_have_extra_entries() {
    let catalog = catalog_utils::load_catalog();
    let descriptor_variants: BTreeSet<_> = descriptors_by_variant().keys().cloned().collect();
    let catalog_variants: BTreeSet<_> = catalog
        .errors
        .iter()
        .map(|entry| entry.variant.clone())
        .collect();

    assert_eq!(
        descriptor_variants, catalog_variants,
        "o catálogo YAML contém variantes extras ou faltantes"
    );
}
