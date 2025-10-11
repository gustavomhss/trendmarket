# Observabilidade 4 — Tracing SDK (Rust)

Este módulo habilita tracing distribuído com OpenTelemetry e integração nativa com `tracing`.
Ele configura exporter OTLP/HTTP, sampler `ParentBased(TraceIdRatio)` e logs JSON com correlação `trace_id`/`span_id`.

## Variáveis de ambiente

| Variável | Default | Descrição |
| --- | --- | --- |
| `SERVICE_NAME` | `credit-engine-core` | Nome lógico do serviço (`service.name`). |
| `SERVICE_VERSION` | `0.1.0` | Versão do serviço (`service.version`). |
| `DEPLOY_ENV` | `local` | Ambiente de implantação (`deployment.environment`). |
| `OTEL_TRACES_SAMPLER` | `parentbased_traceidratio` | Nome do sampler suportado. Outros valores retornam erro. |
| `OTEL_TRACES_SAMPLER_ARG` | `0.1` | Probabilidade (0.0–1.0) usada pelo `TraceIdRatio`. |
| `OTLP_HTTP_URL` | `http://127.0.0.1:4318` | Endpoint base do collector (rota `/v1/traces` adicionada automaticamente). |

O exporter usa cliente `reqwest` com timeout de 5 segundos. Para ambientes com proxy basta configurar as variáveis padrão do `reqwest` (`HTTP_PROXY`, `HTTPS_PROXY`, etc.).

## Uso programático

```rust
use credit_engine_core::obs4::tracing_init::init_tracing;

fn main() -> anyhow::Result<()> {
    let guard = init_tracing()?; // instala propagador W3C e subscriber JSON
    // ... código instrumentado com tracing ...
    guard.shutdown()?; // flush e restaura provider anterior
    Ok(())
}
```

A chamada é idempotente: múltiplas invocações simultâneas retornam `TracingInitError::AlreadyInitialized`.

Os logs emitidos via `tracing` são JSON com campos `timestamp`, `level`, `target`, `trace_id` e `span_id` sempre presentes.

## Smoke test

1. Garanta que o collector esteja ouvindo OTLP/HTTP (porta `4318`).
2. Execute:

```bash
cargo run --quiet --bin obs4_trace_smoke | tee out/obs_gatecheck/logs/obs4_trace_smoke.txt
```

3. Colete a evidência principal:

```bash
grep 'trace_id' out/obs_gatecheck/logs/obs4_trace_smoke.txt \
  > out/obs_gatecheck/evidence/obs4_trace_smoke.log.jsonl
```

O binário gera três spans (`pricing.quote`, `amm.swap` OK, `amm.swap` ERROR) com eventos de log correlacionados.
O guard finaliza o tracer provider para flush explícito.
