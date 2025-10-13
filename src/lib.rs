/* lib (CRD-7-10 FINAL) */
pub mod amm; // existe
pub mod ce_core; // expõe o namespace ce_core

pub mod obs;
pub mod obs4;
pub mod obs_policy_lints;
pub mod otlp_exporter;
// Telemetry modules (auto-exported from src diagnostics)
pub mod telemetry;
pub mod telemetry_cfg;
pub mod telemetry_contract;
pub mod telemetry_identity;
pub mod telemetry_instruments;
pub mod telemetry_latency;
pub mod telemetry_logs;
pub mod telemetry_metrics_otlp;
pub mod telemetry_metrics_prom;
pub mod telemetry_spans_amm;
pub mod telemetry_spans_cdc;
pub mod telemetry_trace;
