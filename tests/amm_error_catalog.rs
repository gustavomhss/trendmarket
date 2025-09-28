use std::collections::{BTreeMap, BTreeSet};

use credit_engine_core::amm::errors::{AmmError, AmmErrorDescriptor, AMM_ERROR_DESCRIPTORS};

#[path = "catalog_utils.rs"]
mod catalog_utils;

fn descriptor_map() -> BTreeMap<String, &'static AmmErrorDescriptor> {
    AMM_ERROR_DESCRIPTORS
        .iter()
        .map(|descriptor| (descriptor.variant.variant_name().to_string(), descriptor))
        .collect()
}

#[test]
fn catalog_is_structurally_valid() {
    let catalog = catalog_utils::load_catalog();

    assert_eq!(catalog.meta.domain, "AMM");
    assert_eq!(catalog.meta.prefix, "CE-AMM");
    assert_eq!(catalog.meta.version, 1);

    let mut seen_codes = BTreeSet::new();
    for entry in &catalog.errors {
        assert!(
            seen_codes.insert(entry.code.clone()),
            "código duplicado: {}",
            entry.code
        );
    }
}

#[test]
fn catalog_matches_runtime_descriptors() {
    let catalog = catalog_utils::load_catalog();
    let descriptors = descriptor_map();

    assert_eq!(
        catalog.errors.len(),
        descriptors.len(),
        "quantidade de entradas divergente"
    );

    for entry in &catalog.errors {
        let descriptor = descriptors
            .get(&entry.variant)
            .unwrap_or_else(|| panic!("descriptor ausente para {}", entry.variant));

        assert_eq!(
            descriptor.code, entry.code,
            "code divergente para {}",
            entry.variant
        );
        assert_eq!(
            descriptor.message, entry.message,
            "mensagem divergente para {}",
            entry.variant
        );
        assert_eq!(
            descriptor.http_status, entry.http_status,
            "http_status divergente para {}",
            entry.variant
        );
    }

    for variant in AmmError::ALL_VARIANTS {
        let variant_name = variant.variant_name();
        assert!(
            catalog
                .errors
                .iter()
                .any(|entry| entry.variant == variant_name),
            "variant {variant_name} ausente do catálogo"
        );
    }
}
