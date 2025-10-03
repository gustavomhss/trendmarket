# OBS-1 — Guia Operacional do Tracing

## Resumo rápido

- **Módulo**: `telemetry_trace`
- **Artefatos principais**: `TraceGuard` (RAII) + `OpenTelemetryLayer` para o `tracing`.
- **Exportador**: OTLP (gRPC ou HTTP/protobuf) com `BatchSpanProcessor`.
- **Propagação**: W3C TraceContext + W3C Baggage (registro global idempotente).

## Inicialização canônica

```rust
use credit_engine_core::telemetry_trace::{init_tracing, ObsLevel, OtlpProtocol, TraceConfig};

let cfg = TraceConfig {
    level: ObsLevel::Min,
    otlp_endpoint: Some("http://otel-collector.stg.svc:4317".into()),
    protocol: Some(OtlpProtocol::Grpc),
    export_timeout_ms: 10_000,
    max_queue_size: 4_096,
    scheduled_delay_ms: 3_000,
    max_export_batch_size: 512,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "1.2.3-stg".into()),
    ("deployment.environment", "stg".into()),
];
let (guard, layer) = init_tracing(cfg, resource)?;
let subscriber = tracing_subscriber::registry().with(layer);
tracing::subscriber::set_global_default(subscriber)?;
// ... aplicação ...
// guard.drop() => shutdown automático
```

## Parâmetros recomendados

| Ambiente | `level`        | Endpoint sugerido                             | Protocolo | Fila (`max_queue_size`) | Delay (`scheduled_delay_ms`) | Observações |
|----------|----------------|-----------------------------------------------|-----------|-------------------------|------------------------------|-------------|
| Dev      | `Off`          | `None`                                        | `None`    | 2_048                   | 5_000                        | Sem Collector, spans no-op |
| Staging  | `Min`          | `http://otel-collector.stg.svc:4317`          | `Grpc`    | 4_096                   | 3_000                        | Sampling 1%, ideal para validação funcional |
| Produção | `Full`         | `https://otel-collector.prod:4318/v1/traces`  | Auto (`Http`) | 8_192               | 2_000                        | Coleta completa com HTTP + TLS |

> **Nota:** Ajuste `max_export_batch_size` (default 512) e `export_timeout_ms` (default 10s) conforme SLOs do Collector.

## Protocolos suportados

- **OTLP/gRPC** (`OtlpProtocol::Grpc`): porta 4317, canal `tonic` com Rustls. Útil quando Collector está dentro da malha e latência é crítica.
- **OTLP/HTTP** (`OtlpProtocol::Http`): porta 4318 ou endpoints `/v1/traces`. Suporte nativo a HTTPS via `reqwest` + Rustls WebPKI.
- **Autodetecção** (`select_protocol`) usa `:4318` ou `/v1/traces` para inferir HTTP; demais casos caem em gRPC.

## Propagação

- Registro único de `TraceContextPropagator` + `BaggagePropagator`.
- `TraceGuard` pode ser inicializado múltiplas vezes: registro é idempotente.
- B3 não é habilitado por padrão; documente caso habilite manualmente no subscriber global.

## Performance & tuning

- `max_queue_size`: aumenta resiliência a burst de spans. Monitorar drop rate no shutdown (`TraceGuard` loga via `eprintln!`).
- `scheduled_delay_ms`: reduzir para diminuir latência de export; aumentar para economizar CPU do Collector.
- `max_export_batch_size`: mantenha ≤ `max_queue_size`. Valores maiores melhoram throughput a custo de latência.
- `export_timeout_ms`: tempo máximo aguardado por lote; alinhar com SLAs do Collector (default 10s).

## Troubleshooting rápido

| Sintoma                               | Mitigação                                                                 |
|--------------------------------------|----------------------------------------------------------------------------|
| `TraceInitError::InvalidResource`    | Verificar se `service.name`, `service.version` e `deployment.environment` estão presentes e não vazios. |
| `TraceInitError::MissingEndpointForActiveLevel` | Informar `otlp_endpoint` quando `level` ∈ {`Min`, `Full`}.                   |
| `TraceInitError::OtlpBuildError`     | Checar URL, esquema (`http`/`https`), certificados Rustls e DNS interno.   |
| Spans dropados na saída              | Aumentar fila ou reduzir `scheduled_delay_ms`; investigar saturação do Collector. |
| Falhas TLS                           | Garantir que o endpoint HTTPS possua cadeias confiáveis (Rustls WebPKI já incluído). |

## Encerramento limpo

- `TraceGuard::shutdown()` pode ser chamado explicitamente (sincrono, chama `SdkTracerProvider::shutdown`).
- `Drop` do guard garante flush idempotente; `AlreadyShutdown` é silenciosamente ignorado.
- Recomenda-se chamar `shutdown()` em ganchos de término (`Ctrl+C`, `SIGTERM`) para evitar perda de spans em lote.

## Integração futura

- Thread 7 irá combinar `telemetry_trace` com a camada de logging estruturado.
- Métricas (Thread 6) usarão configuração semelhante para OTLP Metrics.

## Contatos

- **Owner**: Plataforma / Observabilidade.
- **Fallback**: usar modo `Off` caso Collector esteja indisponível e registrar incidente A110 correspondente.
