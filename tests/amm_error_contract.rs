use credit_engine_core::amm::errors::AmmError;
use std::collections::HashSet;

#[test]
fn amm_error_contract_is_exhaustive_and_well_formed() {
    let descriptors = AmmError::descriptors();
    assert_eq!(descriptors.len(), AmmError::ALL_VARIANTS.len());

    let mut codes = HashSet::new();
    let mut variants = HashSet::new();
    for descriptor in descriptors.iter() {
        let variant = descriptor.variant;
        variants.insert(variant.variant_name());
        let code = descriptor.code;
        assert!(code.starts_with("CE-AMM-"), "code prefix incorrect: {code}");
        assert_eq!(code.len(), 11, "code length incorrect: {code}");
        assert!(codes.insert(code), "duplicate code detected: {code}");

        let message = descriptor.message;
        assert!(
            !message.trim().is_empty(),
            "user message must be non-empty for {}",
            variant.variant_name()
        );
        assert!(
            message.ends_with('.'),
            "user message must end with period: {message}"
        );

        if let Some(status) = descriptor.http_status {
            match status {
                400 | 403 | 404 | 409 | 500 | 502 | 503 => {}
                other => panic!(
                    "unexpected http status {other} for {}",
                    variant.variant_name()
                ),
            }
        }

        assert_eq!(variant.error_code(), code);
        assert_eq!(variant.user_message(), message);
        assert_eq!(variant.http_status(), descriptor.http_status);
    }

    for variant in AmmError::ALL_VARIANTS.iter() {
        assert!(
            variants.contains(variant.variant_name()),
            "variant {} missing from descriptor list",
            variant.variant_name()
        );
    }
}
