# OBS-1 Metrics OTLP Module – Gatecheck README

This README captures the operational view of the OTLP metrics module delivered in OBS-1 Thread 5. The module exposes a focused API to build an OTLP-backed `SdkMeterProvider`, retrieve named meters, and ensure graceful shutdown via RAII.

## Quick Start

```rust
use credit_engine_core::telemetry_metrics_otlp::{
    init_meter_otlp, named_meter, MetricsOtlpConfig, ObsLevel,
};

let cfg = MetricsOtlpConfig {
    level: ObsLevel::Min,
    otlp_endpoint: Some("http://otel-collector.stg:4318/v1/metrics".into()),
    protocol: None,
    export_interval_ms: 5_000,
    export_timeout_ms: 10_000,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "1.7.0-stg".into()),
    ("deployment.environment", "stg".into()),
];
let (mut guard, provider) = init_meter_otlp(cfg, resource)?;
let meter = named_meter(&provider, "ce-amm-stg");
// register instruments (Thread 8) and record metrics here.
// guard.shutdown() optional – drop handles flush.
```

## Configuration Summary

| Field | Default | Notes |
| ----- | ------- | ----- |
| `level` | `ObsLevel::Off` | Off = no exports; `Min`/`Full` require endpoint in non-dev. |
| `otlp_endpoint` | `None` | Required for `Min`/`Full` outside dev. Accepts `http://` or `https://`. |
| `protocol` | Auto | Override when using non-standard ports; autodetection covers 4317/4318 and `/v1/metrics`. |
| `export_interval_ms` | `5_000` | Tune downwards for higher frequency; keep ≥1000ms to avoid collector saturation. |
| `export_timeout_ms` | `10_000` | Exporter timeout per batch. Align with collector SLA. |

Resource metadata is strict: provide **exactly** `service.name`, `service.version`, and `deployment.environment` with non-empty values.

## Environment Guidance

| Environment | Recommended Config | Rationale |
| ----------- | ------------------ | --------- |
| Local / CI | `ObsLevel::Off`, no endpoint | Allows deterministic tests and avoids noisy collector dependencies. |
| Staging | `ObsLevel::Min`, HTTP auto-detect (`4318`) | Validates pipeline with minimal load. Keep TLS off only for isolated staging networks. |
| Production | `ObsLevel::Full`, explicit `OtlpProtocol::Grpc`, TLS endpoint on `4317` | Enable crate feature `metrics-otlp-grpc` to activate the gRPC exporter; ensures high-fidelity metrics with encrypted transport. |

## Operational Notes

* `MetricsGuard` should be held until shutdown to guarantee final flush. Dropping the guard (or calling `shutdown()`) triggers `SdkMeterProvider::shutdown()` once.
* Timeout breaches emit warnings but never panic. Adjust `export_timeout_ms` alongside collector tuning.
* Combine this module with Thread 6 (`/metrics`) and Thread 8 (instrument registration) for full OBS-1 coverage.
* Use the optional crate feature `metrics-otlp-grpc` when collectors require OTLP/gRPC.

## Performance & Cost Considerations

* Shorter intervals increase collector load and network traffic; monitor collector CPU/memory before tuning below 2s.
* gRPC (`4317`) offers lower overhead in production but requires TLS certificates and potential ALB configuration.
* HTTP (`4318`) is simpler for staging and air-gapped environments but may incur higher per-request latency.

## Validation Checklist

- [x] Unit tests cover protocol selection, resource validation, periodic exports via in-memory exporter, and guard shutdown semantics.
- [x] `cargo test` passes with the new feature set (`experimental_metrics_periodicreader_with_async_runtime`).
- [x] Evidence JSON (see `out/obs_gatecheck/evidence/obs1_metrics_otlp_report.json`) records configuration defaults, timestamps, and SHA256 hashes.

