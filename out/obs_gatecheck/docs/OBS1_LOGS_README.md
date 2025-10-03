# OBS-1 Logging Module (Thread 7)

## Montagem do Registry
```rust
use credit_engine_core::telemetry_logs::{json_layer, level_filter, LogConfig};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

let cfg = LogConfig {
    level: "info".into(),
    service: "ce-amm".into(),
    env: "dev".into(),
    version: "2.4.0+1a2b3c4".into(),
};

let layer = json_layer(&cfg)?;
let filter = level_filter(&cfg.level)?;
let subscriber = Registry::default().with(layer).with(filter);
tracing::subscriber::set_global_default(subscriber)?;
```

## Definindo `op`
Use spans ou eventos com o campo `op`:
```rust
let span = tracing::info_span!("pricing", op = "pricing");
let _g = span.enter();
tracing::info!(op = "pricing", "pricing completed");
```
Valores fora de `swap|add_liquidity|remove_liquidity|pricing|cdc_consume` são descartados.

## Correlação com OpenTelemetry
Inclua a layer do tracer quando disponível:
```rust
use opentelemetry_sdk::trace::TracerProvider;
use tracing_subscriber::prelude::*;

let provider = TracerProvider::builder().build();
let tracer = provider.tracer("obs1-sdk", None);
let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
let subscriber = Registry::default().with(otel_layer).with(json_layer(&cfg)?).with(level_filter("info")?);
tracing::subscriber::set_default(subscriber);
```
Dentro de um span válido os campos `trace_id` e `span_id` aparecem automaticamente.

## Checklist rápido
- Todos os eventos exibem `ts`, `level`, `msg`, `service`, `env`, `version`.
- Campos PII (`email`, `cpf`, `phone`, `address`, `name`, `geo`, `person_*`) são ignorados.
- `env` deve ser `dev|stg|prod` (minúsculas) e `level` deve ser um dos níveis suportados.
- Execute `cargo test telemetry_logs_tests` antes de publicar.

## Troubleshooting
- **Sem `trace_id`**: verifique se o span atual foi criado após registrar `tracing_opentelemetry::layer()` e se está ativo (`span.enter()`).
- **Campo removido**: o formatter bloqueia PII e placeholders (`TBD`, `FIXME`, `…`, `PLACEHOLDER`).
- **Mensagem substituída por `[blocked]`**: forneça mensagem válida no evento ou span.
