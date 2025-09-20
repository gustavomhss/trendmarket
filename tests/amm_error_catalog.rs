use std::collections::HashSet;

use credit_engine_core::amm::error::AmmError;
use credit_engine_core::amm::error_catalog::{default_locale_message, AmmErrorCode};

#[test]
fn all_codes_are_unique() {
    let mut seen = HashSet::new();
    for code in AmmErrorCode::all() {
        assert!(seen.insert(code.code()));
    }
    assert_eq!(seen.len(), AmmErrorCode::all().len());
}

#[test]
fn all_messages_nonempty() {
    for code in AmmErrorCode::all() {
        let message = code.message_pt().trim();
        assert!(
            !message.is_empty(),
            "{} message should not be empty",
            code.code()
        );
    }
}

#[test]
fn exhaustive_all_slice() {
    assert_eq!(AmmErrorCode::all().len(), 5);
}

#[test]
fn format_examples_resolve_placeholders() {
    let err = AmmError::new(AmmErrorCode::ZeroAmount).with_context("amount", "0");
    let user = err.to_user_string();
    assert!(user.contains("AMM-0001"));
    let json = err.to_log_json();
    assert!(json.contains("\"context\":{\"amount\":\"0\"}"));
    assert_eq!(
        default_locale_message(AmmErrorCode::ZeroAmount),
        "amount deve ser > 0"
    );
}