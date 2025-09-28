use credit_engine_core::amm::errors::AmmError;
use std::fs;

fn expected_catalog() -> Vec<(AmmError, &'static str, &'static str)> {
    vec![
        (
            AmmError::ZeroAmount,
            "CE-AMM-0001",
            "default_message: \"amount deve ser > 0\"",
        ),
        (
            AmmError::ZeroReserve,
            "CE-AMM-0002",
            "default_message: \"reserve deve ser > 0\"",
        ),
        (
            AmmError::MinReserveBreached,
            "CE-AMM-0003",
            "default_message: \"reserva ficaria abaixo do mínimo\"",
        ),
        (
            AmmError::Overflow,
            "CE-AMM-0004",
            "default_message: \"overflow/underflow numérico\"",
        ),
        (
            AmmError::InputTooSmall,
            "CE-AMM-0005",
            "default_message: \"input efetivo após taxa é 0\"",
        ),
        (
            AmmError::InvalidFee,
            "CE-AMM-0006",
            "default_message: \"fee_ppm deve ser ≤ 1e6\"",
        ),
    ]
}

#[test]
fn catalog_covers_all_amm_errors() {
    let catalog = fs::read_to_string("ops/errors/catalog_amm.yaml").expect("catalog readable");

    for (variant, code, message_line) in expected_catalog() {
        let variant_name = format!("variant: {:?}", variant);
        assert!(
            catalog.contains(&variant_name),
            "missing variant mapping for {}",
            variant_name
        );
        let code_line = format!("code: \"{}\"", code);
        assert!(
            catalog.contains(&code_line),
            "missing code mapping for {}",
            variant_name
        );
        assert!(
            catalog.contains(message_line),
            "missing default message for {}",
            variant_name
        );
    }

    let declared = catalog.matches("variant:").count();
    let expected = expected_catalog().len();
    assert_eq!(
        declared, expected,
        "catalog has {} entries but {} variants are expected",
        declared, expected
    );
}

#[test]
fn http_status_are_present() {
    let catalog = fs::read_to_string("ops/errors/catalog_amm.yaml").expect("catalog readable");
    for status in [400, 409, 500, 422] {
        let needle = format!("http_status: {}", status);
        assert!(
            catalog.contains(&needle),
            "expected to find {} in catalog",
            needle
        );
    }
}
