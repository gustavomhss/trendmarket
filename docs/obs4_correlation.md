# OBS-4 — Correlação ponta-a-ponta

Este módulo adiciona *helpers* para capturar `trace_id`/`span_id` e reforça o vínculo entre
traces, logs estruturados e métricas de latência.

## Logging correlacionado

Use [`log_with_trace`](../src/obs4/correlation.rs) sempre que emitir logs manuais. O helper
aceita um `serde_json::Value` (idealmente um objeto) e, se houver um span ativo, enriquece a
linha com `trace_id` e `span_id` (hexadecimais). Fora de um span, o JSON continua válido,
apenas sem os IDs.

```rust
use credit_engine_core::obs4::correlation::log_with_trace;
use serde_json::json;

log_with_trace(json!({
    "event": "swap_ok",
    "latency_ms": 12.4,
}));
```

### Consulta em Loki

1. Execute o binário de demonstração para gerar logs de exemplo:
   ```bash
   cargo run -q --bin obs4_correlation_demo
   ```
2. Envie os logs para o Loki (ou aguarde o coletor local) e filtre por `trace_id`:
   ```logql
   {service="credit-engine-core"} |= "trace_id" |= "swap_ok"
   ```
   O valor de `trace_id` pode ser utilizado diretamente em Grafana Tempo/Jaeger para abrir o
   trace correspondente.

## Métricas e exemplars

O SDK de métricas (`opentelemetry::metrics::Histogram`) ainda não expõe API para exemplars ou
*trace attachment*. O helper [`observe_with_trace`](../src/obs4/correlation.rs) está
preparado para receber essa funcionalidade, mas, no momento, apenas registra o valor na
métrica sem adicionar rótulos adicionais. A correlação, portanto, permanece via logs.

Quando o suporte a exemplars estiver disponível, atualizaremos o helper para anexar o
`trace_id` como exemplar ou atributo adicional.
