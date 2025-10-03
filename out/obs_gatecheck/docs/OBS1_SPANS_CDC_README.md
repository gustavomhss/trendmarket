# OBS-1 Gatecheck — Helpers `telemetry_spans_cdc`

Este README explica como instrumentar consumo de CDC com os helpers da thread OBS-1/T11.

## 1. API disponível

```rust
use credit_engine_core::telemetry_spans_cdc::{CdcConsumeAttrs, in_cdc_consume, span_cdc_consume};
```

### 1.1 RAII (`span_cdc_consume`)

Retorna `tracing::Span` válido enquanto estiver em escopo. Ideal para integrar com frameworks que já manipulam `Span` diretamente.

### 1.2 Wrapper (`in_cdc_consume`)

Executa uma closure dentro do span e devolve o resultado:

```rust
let result = in_cdc_consume(&attrs, || process_batch());
```

## 2. Preparando o tracer provider

Os helpers não inicializam tracer global. Para testes locais ou serviços que já possuem `tracing` configurado, conecte o provider manualmente:

```rust
use opentelemetry_sdk::trace::{InMemorySpanExporter, SdkTracerProvider};
use tracing_subscriber::{layer::SubscriberExt, Registry};

let exporter = InMemorySpanExporter::default();
let provider = SdkTracerProvider::builder()
    .with_simple_exporter(exporter)
    .build();
let tracer = provider.tracer("cdc-consumer", None);
let layer = tracing_opentelemetry::layer().with_tracer(tracer);
let subscriber = Registry::default().with(layer);
let _guard = tracing::subscriber::set_default(subscriber);
```

A thread OBS-1/T4 decidirá como expor isso globalmente; por ora conecte localmente conforme acima.

## 3. Atributos obrigatórios

- `op = "cdc_consume"`
- `cdc.stream` (`^[a-z0-9._-]{3,64}$`)
- `cdc.partition` (`^[a-zA-Z0-9._-]{1,32}$`)
- `cdc.offset_before` (`>= -1`)
- `cdc.offset_after` (`>= offset_before`)
- `cdc.records` (`>= 0`)
- `cdc.lag_seconds` (`>= 0`, finito)

## 4. Boas práticas de cardinalidade

- Streams: nomes curtos e estáveis (`trades`, `balances_eu`).
- Partições: seguir padrão do broker (`p0`, `p1`, `shard-1`).
- Evitar IDs dinâmicos ou tokens únicos nas strings.
- Caso haja reprocessamento, mantenha `stream` igual e use offsets coerentes.

## 5. Validação e mensagens de erro

Entrada inválida gera `panic!` com mensagens como `invalid cdc.consume attribute \\`cdc.stream\\``. Corrija a origem dos dados antes de tentar abrir o span.

## 6. Fluxo sugerido

1. Calcule offsets e contagem de registros antes de abrir o span.
2. Popule `CdcConsumeAttrs` com dados limpos.
3. Abra o span via RAII ou wrapper.
4. Execute lógica de consumo/ack dentro do span.
5. Ao final, deixe o span sair de escopo; o exportador cuidará da emissão.

## 7. Testes

Execute `cargo test -- --nocapture telemetry_spans_cdc_tests` para validar o comportamento com exportador in-memory.
