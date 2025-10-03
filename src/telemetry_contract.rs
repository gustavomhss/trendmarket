//! OBS-1 canonical telemetry contract constants.

pub const OBS1_CONTRACT_VERSION: &str = "1.0.0";

// Metric names
pub const METRIC_AMM_OP_LATENCY_SECONDS: &str = "amm_op_latency_seconds";
pub const METRIC_HOOK_EXECUTIONS_TOTAL: &str = "hook_executions_total";
pub const METRIC_DATA_FRESHNESS_SECONDS: &str = "data_freshness_seconds";
pub const METRIC_CDC_LAG_SECONDS: &str = "cdc_lag_seconds";
pub const METRIC_DRIFT_SCORE: &str = "drift_score";

// Histogram buckets (seconds)
pub const AMM_OP_LATENCY_BUCKETS: &[f64] = &[
    0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0,
];

// Allowed label keys
pub const LABELS_PERMITIDOS: &[&str] = &[
    "op",
    "service",
    "env",
    "version",
    "hook_id",
    "status",
    "source",
    "domain",
    "stream",
    "partition",
    "feature",
];

// Forbidden label keys
pub const LABELS_PROIBIDOS: &[&str] = &[
    "user_id",
    "account_id",
    "request_id",
    "session_id",
    "*_uuid",
    "*_hash",
];

// Counter label values
pub const HOOK_EXECUTION_STATUS_VALUES: &[&str] = &["success", "error"];

// Span names
pub const SPAN_AMM_SWAP: &str = "amm.swap";
pub const SPAN_AMM_ADD_LIQUIDITY: &str = "amm.add_liquidity";
pub const SPAN_AMM_REMOVE_LIQUIDITY: &str = "amm.remove_liquidity";
pub const SPAN_PRICING_QUOTE: &str = "pricing.quote";
pub const SPAN_CDC_CONSUME: &str = "cdc.consume";

pub const SPAN_NAMES: &[&str] = &[
    SPAN_AMM_SWAP,
    SPAN_AMM_ADD_LIQUIDITY,
    SPAN_AMM_REMOVE_LIQUIDITY,
    SPAN_PRICING_QUOTE,
    SPAN_CDC_CONSUME,
];

// Span attributes
pub const SPAN_ATTR_AMM_K_BEFORE: &str = "amm.k_before";
pub const SPAN_ATTR_AMM_K_AFTER: &str = "amm.k_after";
pub const SPAN_ATTR_AMM_DELTA_K_RATIO: &str = "amm.delta_k_ratio";
pub const SPAN_ATTR_AMM_FEE_PPM: &str = "amm.fee_ppm";
pub const SPAN_ATTR_AMM_INPUT: &str = "amm.input";
pub const SPAN_ATTR_AMM_OUTPUT: &str = "amm.output";

pub const SPAN_REQUIRED_ATTRIBUTES: &[&str] = &[
    SPAN_ATTR_AMM_K_BEFORE,
    SPAN_ATTR_AMM_K_AFTER,
    SPAN_ATTR_AMM_DELTA_K_RATIO,
    SPAN_ATTR_AMM_FEE_PPM,
    SPAN_ATTR_AMM_INPUT,
    SPAN_ATTR_AMM_OUTPUT,
];

// Operation mapping shared by spans, metrics and logs
pub const OPERATION_VALUES: &[&str] = &[
    "swap",
    "add_liquidity",
    "remove_liquidity",
    "pricing",
    "cdc_consume",
];

pub const OPERATION_REGEX: &str = "^(swap|add_liquidity|remove_liquidity|pricing|cdc_consume)$";
pub const LABEL_REGEX: &str = "^[a-z0-9_]{1,32}$";

// Log field names
pub const LOG_FIELD_TIMESTAMP: &str = "ts";
pub const LOG_FIELD_LEVEL: &str = "level";
pub const LOG_FIELD_MESSAGE: &str = "msg";
pub const LOG_FIELD_TRACE_ID: &str = "trace_id";
pub const LOG_FIELD_SPAN_ID: &str = "span_id";
pub const LOG_FIELD_SERVICE: &str = "service";
pub const LOG_FIELD_ENV: &str = "env";
pub const LOG_FIELD_OPERATION: &str = "op";
pub const LOG_FIELD_VERSION: &str = "version";
pub const LOG_FIELD_HOOK_ID: &str = "hook_id";
pub const LOG_FIELD_ERROR_KIND: &str = "error.kind";
pub const LOG_FIELD_ERROR_MESSAGE: &str = "error.message";
pub const LOG_FIELD_EXTRA: &str = "extra";

pub const LOG_LEVEL_VALUES: &[&str] = &["trace", "debug", "info", "warn", "error"];
pub const LOG_TIMESTAMP_PATTERN_UTC: &str = "^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}Z$";
pub const LOG_TRACE_ID_PATTERN: &str = "^[0-9a-f]{32}$";
pub const LOG_SPAN_ID_PATTERN: &str = "^[0-9a-f]{16}$";
pub const LOG_VERSION_PATTERN: &str =
    "^(?:\\d+\\.\\d+\\.\\d+(?:[-+][0-9A-Za-z.-]+)?|[0-9a-f]{7,40})$";
pub const LOG_HOOK_ID_PATTERN: &str = "^[a-z0-9]+(?:[-_][a-z0-9]+)*$";

// Resource attributes
pub const RESOURCE_SERVICE_NAME: &str = "service.name";
pub const RESOURCE_SERVICE_VERSION: &str = "service.version";
pub const RESOURCE_DEPLOYMENT_ENVIRONMENT: &str = "deployment.environment";

pub const RESOURCE_SERVICE_NAME_VALUE: &str = "ce-amm";
pub const RESOURCE_ENV_VALUES: &[&str] = &["dev", "stg", "prod"];

// Flags and environment variables
pub const ENV_OBSERVABILITY_LEVEL: &str = "OBSERVABILITY_LEVEL";
pub const ENV_PROM_SCRAPE: &str = "PROM_SCRAPE";
pub const ENV_OTEL_EXPORTER_OTLP_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";

pub const OBSERVABILITY_LEVEL_VALUES: &[&str] = &["off", "min", "full"];
pub const PROM_SCRAPE_VALUES: &[&str] = &["on", "off"];

// PII forbidden keys
pub const PII_FORBIDDEN_FIELDS: &[&str] = &["email", "cpf", "phone", "address", "name", "geo"];

// File manifest for evidence reports
pub const CONTRACT_FILES: &[&str] = &[
    "docs/obs1_contract.md",
    "src/telemetry_contract.rs",
    "schemas/obs1_log_record.schema.json",
    "schemas/obs1_contract.yaml",
    "out/obs_gatecheck/docs/OBS1_CONTRACT_README.md",
    "out/obs_gatecheck/evidence/obs1_contract_report.json",
];
