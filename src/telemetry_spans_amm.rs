use std::fmt;

use tracing::{span, Level, Span};

/// Attributes for canonical `amm.swap` spans.
#[derive(Debug, Clone, PartialEq)]
pub struct SwapAttrs {
    pub k_before: f64,
    pub k_after: f64,
    pub delta_k_ratio: f64,
    pub fee_ppm: i64,
    pub input: f64,
    pub output: f64,
}

/// Attributes for canonical `amm.add_liquidity` spans.
#[derive(Debug, Clone, PartialEq)]
pub struct AddLiquidityAttrs {
    pub k_before: f64,
    pub k_after: f64,
    pub delta_k_ratio: f64,
    pub fee_ppm: i64,
    pub input: f64,
    pub output: f64,
}

/// Attributes for canonical `amm.remove_liquidity` spans.
#[derive(Debug, Clone, PartialEq)]
pub struct RemoveLiquidityAttrs {
    pub k_before: f64,
    pub k_after: f64,
    pub delta_k_ratio: f64,
    pub fee_ppm: i64,
    pub input: f64,
    pub output: f64,
}

/// Attributes for canonical `pricing.quote` spans.
///
/// Although pricing operations may not mutate the invariant `k`,
/// this helper keeps the same attribute set for uniformity across
/// telemetry data, following the OBS-1 contract.
#[derive(Debug, Clone, PartialEq)]
pub struct PricingQuoteAttrs {
    pub k_before: f64,
    pub k_after: f64,
    pub delta_k_ratio: f64,
    pub fee_ppm: i64,
    pub input: f64,
    pub output: f64,
}

#[derive(Debug)]
struct ValidationError {
    op: &'static str,
    field: &'static str,
    message: &'static str,
}

impl ValidationError {
    fn new(op: &'static str, field: &'static str, message: &'static str) -> Self {
        Self { op, field, message }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid telemetry attributes for {}: {} {}",
            self.op, self.field, self.message
        )
    }
}

fn panic_on_invalid(result: Result<(), ValidationError>) {
    if let Err(err) = result {
        panic!("{}", err);
    }
}

fn validate_common(
    op: &'static str,
    k_before: f64,
    k_after: f64,
    delta_k_ratio: f64,
    fee_ppm: i64,
    input: f64,
    output: f64,
) -> Result<(), ValidationError> {
    ensure_finite_positive(op, "amm.k_before", k_before)?;
    ensure_finite_positive(op, "amm.k_after", k_after)?;
    ensure_finite(op, "amm.delta_k_ratio", delta_k_ratio)?;
    if delta_k_ratio.abs() > 1_000_000.0 {
        return Err(ValidationError::new(
            op,
            "amm.delta_k_ratio",
            "absolute value must be ≤ 1e6",
        ));
    }
    ensure_fee(op, fee_ppm)?;
    ensure_non_negative(op, "amm.input", input)?;
    ensure_non_negative(op, "amm.output", output)?;
    Ok(())
}

fn ensure_finite_positive(
    op: &'static str,
    field: &'static str,
    value: f64,
) -> Result<(), ValidationError> {
    ensure_finite(op, field, value)?;
    if value <= 0.0 {
        return Err(ValidationError::new(op, field, "must be > 0"));
    }
    Ok(())
}

fn ensure_non_negative(
    op: &'static str,
    field: &'static str,
    value: f64,
) -> Result<(), ValidationError> {
    ensure_finite(op, field, value)?;
    if value < 0.0 {
        return Err(ValidationError::new(op, field, "must be ≥ 0"));
    }
    Ok(())
}

fn ensure_finite(op: &'static str, field: &'static str, value: f64) -> Result<(), ValidationError> {
    if !value.is_finite() {
        return Err(ValidationError::new(op, field, "must be finite"));
    }
    Ok(())
}

fn ensure_fee(op: &'static str, fee_ppm: i64) -> Result<(), ValidationError> {
    if fee_ppm < 0 {
        return Err(ValidationError::new(op, "amm.fee_ppm", "must be ≥ 0"));
    }
    Ok(())
}

fn build_span(
    name: &'static str,
    op: &'static str,
    k_before: f64,
    k_after: f64,
    delta_k_ratio: f64,
    fee_ppm: i64,
    input: f64,
    output: f64,
) -> Span {
    let span = match name {
        "amm.swap" => span!(
            Level::INFO,
            "amm.swap",
            op = tracing::field::Empty,
            "amm.k_before" = tracing::field::Empty,
            "amm.k_after" = tracing::field::Empty,
            "amm.delta_k_ratio" = tracing::field::Empty,
            "amm.fee_ppm" = tracing::field::Empty,
            "amm.input" = tracing::field::Empty,
            "amm.output" = tracing::field::Empty,
        ),
        "amm.add_liquidity" => span!(
            Level::INFO,
            "amm.add_liquidity",
            op = tracing::field::Empty,
            "amm.k_before" = tracing::field::Empty,
            "amm.k_after" = tracing::field::Empty,
            "amm.delta_k_ratio" = tracing::field::Empty,
            "amm.fee_ppm" = tracing::field::Empty,
            "amm.input" = tracing::field::Empty,
            "amm.output" = tracing::field::Empty,
        ),
        "amm.remove_liquidity" => span!(
            Level::INFO,
            "amm.remove_liquidity",
            op = tracing::field::Empty,
            "amm.k_before" = tracing::field::Empty,
            "amm.k_after" = tracing::field::Empty,
            "amm.delta_k_ratio" = tracing::field::Empty,
            "amm.fee_ppm" = tracing::field::Empty,
            "amm.input" = tracing::field::Empty,
            "amm.output" = tracing::field::Empty,
        ),
        "pricing.quote" => span!(
            Level::INFO,
            "pricing.quote",
            op = tracing::field::Empty,
            "amm.k_before" = tracing::field::Empty,
            "amm.k_after" = tracing::field::Empty,
            "amm.delta_k_ratio" = tracing::field::Empty,
            "amm.fee_ppm" = tracing::field::Empty,
            "amm.input" = tracing::field::Empty,
            "amm.output" = tracing::field::Empty,
        ),
        _ => unreachable!("unexpected span name: {}", name),
    };
    span.record("op", &op);
    span.record("amm.k_before", &k_before);
    span.record("amm.k_after", &k_after);
    span.record("amm.delta_k_ratio", &delta_k_ratio);
    span.record("amm.fee_ppm", &fee_ppm);
    span.record("amm.input", &input);
    span.record("amm.output", &output);
    span
}

/// Creates a canonical span for `amm.swap` with validated attributes.
pub fn span_amm_swap(attrs: &SwapAttrs) -> Span {
    panic_on_invalid(validate_common(
        "amm.swap",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    ));
    build_span(
        "amm.swap",
        "swap",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    )
}

/// Executes `f` within an `amm.swap` span, returning `f`'s output.
pub fn in_amm_swap<F, T>(attrs: &SwapAttrs, f: F) -> T
where
    F: FnOnce() -> T,
{
    let span = span_amm_swap(attrs);
    let _guard = span.enter();
    let result = f();
    drop(_guard);
    result
}

/// Creates a canonical span for `amm.add_liquidity` with validated attributes.
pub fn span_amm_add_liquidity(attrs: &AddLiquidityAttrs) -> Span {
    panic_on_invalid(validate_common(
        "amm.add_liquidity",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    ));
    build_span(
        "amm.add_liquidity",
        "add_liquidity",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    )
}

/// Executes `f` within an `amm.add_liquidity` span, returning `f`'s output.
pub fn in_amm_add_liquidity<F, T>(attrs: &AddLiquidityAttrs, f: F) -> T
where
    F: FnOnce() -> T,
{
    let span = span_amm_add_liquidity(attrs);
    let _guard = span.enter();
    let result = f();
    drop(_guard);
    result
}

/// Creates a canonical span for `amm.remove_liquidity` with validated attributes.
pub fn span_amm_remove_liquidity(attrs: &RemoveLiquidityAttrs) -> Span {
    panic_on_invalid(validate_common(
        "amm.remove_liquidity",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    ));
    build_span(
        "amm.remove_liquidity",
        "remove_liquidity",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    )
}

/// Executes `f` within an `amm.remove_liquidity` span, returning `f`'s output.
pub fn in_amm_remove_liquidity<F, T>(attrs: &RemoveLiquidityAttrs, f: F) -> T
where
    F: FnOnce() -> T,
{
    let span = span_amm_remove_liquidity(attrs);
    let _guard = span.enter();
    let result = f();
    drop(_guard);
    result
}

/// Creates a canonical span for `pricing.quote` with validated attributes.
pub fn span_pricing_quote(attrs: &PricingQuoteAttrs) -> Span {
    panic_on_invalid(validate_common(
        "pricing.quote",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    ));
    build_span(
        "pricing.quote",
        "pricing",
        attrs.k_before,
        attrs.k_after,
        attrs.delta_k_ratio,
        attrs.fee_ppm,
        attrs.input,
        attrs.output,
    )
}

/// Executes `f` within a `pricing.quote` span, returning `f`'s output.
pub fn in_pricing_quote<F, T>(attrs: &PricingQuoteAttrs, f: F) -> T
where
    F: FnOnce() -> T,
{
    let span = span_pricing_quote(attrs);
    let _guard = span.enter();
    let result = f();
    drop(_guard);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_positive_k() {
        let attrs = SwapAttrs {
            k_before: -1.0,
            k_after: 1.0,
            delta_k_ratio: 0.0,
            fee_ppm: 0,
            input: 0.0,
            output: 0.0,
        };

        let err = std::panic::catch_unwind(|| span_amm_swap(&attrs));
        assert!(err.is_err());
    }
}
