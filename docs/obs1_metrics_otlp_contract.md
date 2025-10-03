# OBS-1 Metrics OTLP Contract

This document defines the contract for initializing the OBS-1 metrics pipeline with an OTLP exporter. The goal is to provide a self-contained Rust module that builds an `SdkMeterProvider` backed by a periodic OTLP exporter with clean shutdown semantics and well-defined configuration knobs.

## 1. Configuration Surface

```rust
#[derive(Debug, Clone)]
pub struct MetricsOtlpConfig {
    pub level: ObsLevel,
    pub otlp_endpoint: Option<String>,
    pub protocol: Option<OtlpProtocol>,
    pub export_interval_ms: u64,
    pub export_timeout_ms: u64,
}
```

* **`level`** – observation level (`Off`, `Min`, `Full`). `Off` yields a no-op provider. `Min` and `Full` enable exports and **must** be paired with a reachable OTLP endpoint in staging/production.
* **`otlp_endpoint`** – HTTP or gRPC OTLP collector endpoint. Required when `level != Off` outside of local development.
* **`protocol`** – optional override. When `None`, the module auto-detects based on the endpoint (port `4318` or path `/v1/metrics` ⇒ HTTP, otherwise gRPC).
* **`export_interval_ms`** – flush cadence for the `PeriodicReader`. Default: `5_000` ms.
* **`export_timeout_ms`** – OTLP exporter timeout. Default: `10_000` ms.

### Resource Pairs

`ResourcePairs` must contain **exactly** the canonical semantic attributes:

| Key | Description |
| --- | ----------- |
| `service.name` | Stable identifier of the application |
| `service.version` | Build or release identifier |
| `deployment.environment` | Environment label (`dev`, `stg`, `prod`, …) |

All values must be non-empty. Any missing, duplicated, or unexpected key results in `MetricsInitError::InvalidResource`.

## 2. Behaviour by Observation Level

| Level | Endpoint Required | Exporter Behaviour |
| ----- | ----------------- | ------------------ |
| `Off` | No | Builds an `SdkMeterProvider` without readers (no export, safe for offline/dev) |
| `Min` | Yes (stg/prod) | Configures OTLP exporter with lightweight defaults |
| `Full` | Yes | Same exporter with higher sampling pressure and full fidelity (instrument tuning is handled in OBS-1 T8) |

*In development it is acceptable to keep `otlp_endpoint = None` with `ObsLevel::Off`. In any environment that should emit telemetry, `Min` or `Full` **must** include a valid collector endpoint.*

## 3. Protocol Selection

```rust
pub fn select_protocol(endpoint: &str, explicit: Option<OtlpProtocol>) -> OtlpProtocol
```

1. Respect explicit overrides.
2. Autodetect:
   * Port `4318` or path `/v1/metrics` → `OtlpProtocol::Http`.
   * Otherwise → `OtlpProtocol::Grpc`.
3. Supports TLS endpoints transparently (`https://` URLs are forwarded to the OTLP client).
4. Enabling real gRPC transport requires the crate feature `metrics-otlp-grpc`. Without it, `init_meter_otlp` returns
   `MetricsInitError::OtlpBuildError` for gRPC endpoints.

## 4. Periodic Export & Shutdown

* `init_meter_otlp` wires a `PeriodicReader` with configurable interval and exporter timeout.
* The OTLP exporter timeout is enforced per batch; slow collectors are surfaced as `MetricsInitError::OtlpBuildError` during setup or warnings at runtime.
* `MetricsGuard` implements RAII shutdown – dropping the guard (or calling `shutdown()` explicitly) triggers `SdkMeterProvider::shutdown()` exactly once without panics. Errors are logged but never crash the process.

## 5. Usage Examples

### 5.1 Development – Observation Off

```rust
let cfg = MetricsOtlpConfig {
    level: ObsLevel::Off,
    otlp_endpoint: None,
    protocol: None,
    export_interval_ms: 5_000,
    export_timeout_ms: 10_000,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "0.0.0+devhash".into()),
    ("deployment.environment", "dev".into()),
];
let (guard, provider) = init_meter_otlp(cfg, resource)?; // provider is no-op
let meter = named_meter(&provider, "ce-amm-dev");
```

### 5.2 Staging – HTTP (4318)

```rust
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
let (guard, provider) = init_meter_otlp(cfg, resource)?;
let meter = named_meter(&provider, "ce-amm-stg");
```

### 5.3 Production – gRPC (4317)

```rust
let cfg = MetricsOtlpConfig {
    level: ObsLevel::Full,
    otlp_endpoint: Some("https://otel-collector.prod:4317".into()),
    protocol: Some(OtlpProtocol::Grpc),
    export_interval_ms: 5_000,
    export_timeout_ms: 10_000,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "2.3.4".into()),
    ("deployment.environment", "prod".into()),
];
let (guard, provider) = init_meter_otlp(cfg, resource)?;
let meter = named_meter(&provider, "ce-amm-prod");
```

## 6. Recommended Settings by Environment

| Environment | Level | Protocol | Interval | Timeout | Notes |
| ----------- | ----- | -------- | -------- | ------- | ----- |
| Local/CI | `Off` | n/a | 5s | 10s | Avoids noisy collector failures during dev runs |
| Staging | `Min` | Auto (`4318` → HTTP) | 5s | 10s | Validates connectivity with minimal pressure |
| Production | `Full` | Explicit gRPC (`4317`) | 5s | 10s | Use TLS (`https://`) and collector-side auth |

> For heavy workloads adjust `export_interval_ms` downwards (e.g., 2000 ms) and ensure the collector has capacity. For bursts, keep the timeout aligned with collector SLAs and monitor exporter warnings.

## 7. Troubleshooting

| Symptom | Checks & Remediation |
| ------- | -------------------- |
| **No exports observed** | Confirm `ObsLevel` is `Min`/`Full`, resource keys are correct, and endpoint is set. Review logs for `meter provider shutdown failed` warnings. |
| **High export latency** | Increase `export_timeout_ms` temporarily; validate collector health. Consider shorter intervals to reduce batch size. |
| **Connection refused** | Ensure the OTLP collector is reachable; verify host/port (`4317` for gRPC, `4318` for HTTP). |
| **Wrong protocol** | Use `protocol = Some(...)` to override autodetection when the collector exposes non-standard ports. |
| **TLS errors** | Provide `https://` endpoints with valid certificates. Collector-side mTLS requires configuring the OTLP exporter TLS settings (handled in OBS-1 T6/T8). |
| **Collector path mismatch** | HTTP endpoints must include `/v1/metrics`. Missing path triggers gRPC fallback—set an explicit protocol or include the canonical path. |

## 8. Contract Guarantees

1. `init_meter_otlp` never registers domain instruments (handled in OBS-1 T8).
2. `MetricsGuard` ensures a clean shutdown and never panics.
3. Resource validation is strict, enforcing semantic conventions upfront.
4. Configuration is self-contained—no global state is mutated.
5. Tests cover protocol detection, resource validation, guard shutdown, and periodic export via an in-memory exporter.

