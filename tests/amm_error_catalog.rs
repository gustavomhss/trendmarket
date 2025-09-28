use std::collections::BTreeSet;

use credit_engine_core::amm::errors::AmmError;

#[path = "catalog_utils.rs"]
mod catalog_utils;

#[test]
fn catalog_metadata_is_consistent() {
    let catalog = catalog_utils::load_catalog();

    assert_eq!(catalog.meta.domain, "AMM", "unexpected catalog domain");
    assert_eq!(catalog.meta.prefix, "CE-AMM", "unexpected catalog prefix");
    assert_eq!(catalog.meta.version, 1, "unexpected catalog version");
}

#[test]
fn catalog_entries_match_runtime_descriptors() {
    let catalog = catalog_utils::load_catalog();
    let yaml_map = catalog.entries_by_variant();
    assert_eq!(
        yaml_map.len(),
        catalog.errors.len(),
        "catalog contains duplicated variants"
    );

    let runtime_snapshot = catalog_utils::runtime_catalog_snapshot();
    assert_eq!(
        runtime_snapshot.len(),
        AmmError::ALL_VARIANTS.len(),
        "runtime descriptors and enum variants diverge"
    );

    let yaml_variants: BTreeSet<_> = yaml_map.keys().cloned().collect();
    let runtime_variants: BTreeSet<_> = runtime_snapshot.keys().cloned().collect();
    assert_eq!(
        yaml_variants, runtime_variants,
        "YAML catalog variants diverge from runtime descriptors"
    );

    let yaml_codes: BTreeSet<_> = yaml_map.values().map(|entry| entry.code.clone()).collect();
    assert_eq!(
        yaml_codes.len(),
        yaml_map.len(),
        "catalog contains duplicated error codes"
    );

    assert_eq!(
        yaml_map, runtime_snapshot,
        "YAML catalog contents diverge from runtime descriptors"
    );
}
