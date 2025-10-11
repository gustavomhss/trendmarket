use std::{borrow::Cow, fmt};

use opentelemetry::trace::Status;
use tracing::{event, span, Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

const TARGET_AMM: &str = "obs.amm";
const TARGET_PRICING: &str = "obs.pricing";
const TARGET_CDC: &str = "obs.cdc";
const EVENT_VALIDATION_FAILED: &str = "amm.validation_failed";
const EVENT_ROUNDING_TRACE: &str = "amm.rounding_trace";
const MAX_GUARDRAIL_CODE: usize = 32;
const MAX_GUARDRAIL_REASON: usize = 64;
const MAX_ROUNDING_RULE: usize = 32;

#[derive(Debug, Clone)]
pub struct GuardrailEvent<'a> {
    pub code: &'a str,
    pub reason: &'a str,
}

#[derive(Debug, Clone)]
pub struct AmmReq<'a> {
    pub k_before: f64,
    pub k_after: f64,
    pub delta_k_ratio: f64,
    pub fee_ppm: i64,
    pub input_amount: f64,
    pub output_amount: f64,
    pub asset_in: &'a str,
    pub asset_out: &'a str,
    pub guardrail_hit: bool,
    pub guardrail: Option<GuardrailEvent<'a>>,
    pub rounding_rule: Option<&'a str>,
}

pub type SwapReq<'a> = AmmReq<'a>;
pub type AddReq<'a> = AmmReq<'a>;
pub type RemoveReq<'a> = AmmReq<'a>;
pub type QuoteReq<'a> = AmmReq<'a>;

#[derive(Debug, Clone)]
pub struct CdcMeta<'a> {
    pub stream: &'a str,
    pub partition: &'a str,
    pub offset_before: i64,
    pub offset_after: i64,
    pub records: i64,
    pub lag_seconds: f64,
}

pub fn span_amm_swap(req: &SwapReq<'_>) -> Span {
    let span = span!(
        target: TARGET_AMM,
        Level::INFO,
        "amm.swap",
        "amm.k_before" = tracing::field::Empty,
        "amm.k_after" = tracing::field::Empty,
        "amm.delta_k_ratio" = tracing::field::Empty,
        "amm.fee_ppm" = tracing::field::Empty,
        "amm.input_amount" = tracing::field::Empty,
        "amm.output_amount" = tracing::field::Empty,
        "amm.asset_in" = tracing::field::Empty,
        "amm.asset_out" = tracing::field::Empty,
        "amm.guardrail_hit" = tracing::field::Empty,
    );
    apply_amm_attributes(&span, req, TARGET_AMM);
    span
}

pub fn span_amm_add_liquidity(req: &AddReq<'_>) -> Span {
    let span = span!(
        target: TARGET_AMM,
        Level::INFO,
        "amm.add_liquidity",
        "amm.k_before" = tracing::field::Empty,
        "amm.k_after" = tracing::field::Empty,
        "amm.delta_k_ratio" = tracing::field::Empty,
        "amm.fee_ppm" = tracing::field::Empty,
        "amm.input_amount" = tracing::field::Empty,
        "amm.output_amount" = tracing::field::Empty,
        "amm.asset_in" = tracing::field::Empty,
        "amm.asset_out" = tracing::field::Empty,
        "amm.guardrail_hit" = tracing::field::Empty,
    );
    apply_amm_attributes(&span, req, TARGET_AMM);
    span
}

pub fn span_amm_remove_liquidity(req: &RemoveReq<'_>) -> Span {
    let span = span!(
        target: TARGET_AMM,
        Level::INFO,
        "amm.remove_liquidity",
        "amm.k_before" = tracing::field::Empty,
        "amm.k_after" = tracing::field::Empty,
        "amm.delta_k_ratio" = tracing::field::Empty,
        "amm.fee_ppm" = tracing::field::Empty,
        "amm.input_amount" = tracing::field::Empty,
        "amm.output_amount" = tracing::field::Empty,
        "amm.asset_in" = tracing::field::Empty,
        "amm.asset_out" = tracing::field::Empty,
        "amm.guardrail_hit" = tracing::field::Empty,
    );
    apply_amm_attributes(&span, req, TARGET_AMM);
    span
}

pub fn span_pricing_quote(req: &QuoteReq<'_>) -> Span {
    let span = span!(
        target: TARGET_PRICING,
        Level::INFO,
        "pricing.quote",
        "amm.k_before" = tracing::field::Empty,
        "amm.k_after" = tracing::field::Empty,
        "amm.delta_k_ratio" = tracing::field::Empty,
        "amm.fee_ppm" = tracing::field::Empty,
        "amm.input_amount" = tracing::field::Empty,
        "amm.output_amount" = tracing::field::Empty,
        "amm.asset_in" = tracing::field::Empty,
        "amm.asset_out" = tracing::field::Empty,
        "amm.guardrail_hit" = tracing::field::Empty,
    );
    apply_amm_attributes(&span, req, TARGET_PRICING);
    span
}

pub fn span_cdc_consume(meta: &CdcMeta<'_>) -> Span {
    let span = span!(
        target: TARGET_CDC,
        Level::INFO,
        "cdc.consume",
        "cdc.stream" = tracing::field::Empty,
        "cdc.partition" = tracing::field::Empty,
        "cdc.offset_before" = tracing::field::Empty,
        "cdc.offset_after" = tracing::field::Empty,
        "cdc.records" = tracing::field::Empty,
        "cdc.lag_seconds" = tracing::field::Empty,
    );
    span.record("cdc.stream", meta.stream);
    span.record("cdc.partition", meta.partition);
    span.record("cdc.offset_before", meta.offset_before);
    span.record("cdc.offset_after", meta.offset_after);
    span.record("cdc.records", meta.records);
    span.record("cdc.lag_seconds", meta.lag_seconds);
    span
}

pub fn truncate16(input: &str) -> Cow<'_, str> {
    truncate_to(input, 16)
}

pub fn set_status_from_result<T, E>(span: &Span, result: &Result<T, E>)
where
    E: fmt::Display,
{
    match result {
        Ok(_) => span.set_status(Status::Ok),
        Err(err) => span.set_status(Status::error(err.to_string())),
    }
}

fn apply_amm_attributes(span: &Span, req: &AmmReq<'_>, event_target: &'static str) {
    span.record("amm.k_before", req.k_before);
    span.record("amm.k_after", req.k_after);
    span.record("amm.delta_k_ratio", req.delta_k_ratio);
    span.record("amm.fee_ppm", req.fee_ppm);
    span.record("amm.input_amount", req.input_amount);
    span.record("amm.output_amount", req.output_amount);

    let asset_in = truncate16(req.asset_in);
    let asset_out = truncate16(req.asset_out);
    span.record("amm.asset_in", asset_in.as_ref());
    span.record("amm.asset_out", asset_out.as_ref());

    span.record("amm.guardrail_hit", req.guardrail_hit);

    if let Some(rule) = req.rounding_rule {
        let rule_value = truncate_to(rule, MAX_ROUNDING_RULE);
        emit_rounding_trace(span, event_target, rule_value.as_ref());
    }

    if req.guardrail_hit {
        if let Some(guardrail) = &req.guardrail {
            let code = truncate_to(guardrail.code, MAX_GUARDRAIL_CODE);
            let reason = truncate_to(guardrail.reason, MAX_GUARDRAIL_REASON);
            span.record("amm.guardrail_hit", true);
            emit_guardrail_failed(span, event_target, code.as_ref(), reason.as_ref());
        } else {
            span.record("amm.guardrail_hit", true);
        }
    }
}

fn truncate_to<'a>(input: &'a str, max_len: usize) -> Cow<'a, str> {
    if input.chars().count() <= max_len {
        Cow::Borrowed(input)
    } else {
        Cow::Owned(input.chars().take(max_len).collect())
    }
}

fn emit_rounding_trace(span: &Span, target: &'static str, rule: &str) {
    match target {
        TARGET_PRICING => span.in_scope(|| {
            event!(target: TARGET_PRICING, Level::DEBUG, r_rule = rule, EVENT_ROUNDING_TRACE);
        }),
        _ => span.in_scope(|| {
            event!(target: TARGET_AMM, Level::DEBUG, r_rule = rule, EVENT_ROUNDING_TRACE);
        }),
    }
}

fn emit_guardrail_failed(span: &Span, target: &'static str, code: &str, reason: &str) {
    match target {
        TARGET_PRICING => span.in_scope(|| {
            event!(
                target: TARGET_PRICING,
                Level::WARN,
                code = code,
                reason = reason,
                EVENT_VALIDATION_FAILED
            );
        }),
        _ => span.in_scope(|| {
            event!(
                target: TARGET_AMM,
                Level::WARN,
                code = code,
                reason = reason,
                EVENT_VALIDATION_FAILED
            );
        }),
    }
}
