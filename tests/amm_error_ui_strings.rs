use credit_engine_core::amm::error::AmmError;
use credit_engine_core::amm::error_catalog::AmmErrorCode;

#[test]
fn no_newlines_or_tabs() {
    let err =
        AmmError::new(AmmErrorCode::ZeroReserve).with_context("origem", "linha1\nlinha2\ttab");
    let user = err.to_user_string();
    assert!(!user.contains('\n'));
    assert!(!user.contains('\t'));
}

#[test]
fn truncate_long_context_values() {
    let long_value = "a".repeat(1024);
    let err = AmmError::new(AmmErrorCode::OverflowNumeric).with_context("detalhe", long_value);
    let user = err.to_user_string();
    assert!(user.len() < 512);
}

#[test]
fn unknown_placeholder_is_left_as_is() {
    let err = AmmError::new(AmmErrorCode::ZeroAmount);
    let rendered = err.render_with_template("erro {desconhecido}");
    assert_eq!(rendered, "erro {desconhecido}");
}
