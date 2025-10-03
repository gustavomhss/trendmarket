# OBS-1 / `obs_demo` – Guia de Operação Rápido

## Comandos úteis

### Prometheus local (DEV)
```bash
export DEPLOY_ENV=dev
export OBSERVABILITY_LEVEL=full
export PROM_SCRAPE=on
export METRICS_HTTP_ADDR=127.0.0.1:9464
RUST_LOG=info cargo run --bin obs_demo
```
- Abre `/metrics` em `http://127.0.0.1:9464/metrics` com `amm_op_latency_seconds` e `hook_executions_total`.
- Logs estruturados JSON são impressos no stdout com `trace_id`/`span_id`.

### OTLP (STG/PROD)
```bash
export DEPLOY_ENV=stg
export OBSERVABILITY_LEVEL=min
export OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector.stg:4318/v1/metrics
RUST_LOG=info cargo run --bin obs_demo
```
- Exporta spans/metrics via OTLP HTTP.
- `/metrics` permanece desligado (a menos que `PROM_SCRAPE=on`).

### Execução enxuta
```bash
OBS_DEMO_OPS=5 PROM_SCRAPE=off cargo run --bin obs_demo
```
- Útil para smoke tests; apenas imprime logs e resumo final.

## Variáveis suportadas

| Variável | Default | Observações |
| --- | --- | --- |
| `OBS_DEMO_OPS` | `20` | Número de operações sintéticas (swap → quote → cdc). |
| `PROM_SCRAPE` | `off` | `on` abre `/metrics`. Exige `METRICS_HTTP_ADDR`. |
| `METRICS_HTTP_ADDR` | `0.0.0.0:9464` | Listener Prometheus. Altere se a porta estiver ocupada. |
| `OBSERVABILITY_LEVEL` | `min` | `off` desliga exportadores; `full` habilita logs + tracing + métricas. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | — | Obrigatório em `prod`. Formato `http(s)://host:port[/v1/... ]`. |
| `LOG_LEVEL` | `info` | Respeita `RUST_LOG`; manter `info` para logs canônicos. |

## Tráfego sintético
- **AMM swap**: CPMM determinístico (`fee_ppm` 100–500, `ε` ±5%). Emite hook `amm.risk-check`.
- **Pricing quote**: Recalcula preços com `ε` ±2% sem alterar reservas base.
- **CDC consume**: Streams `trades`/`quotes`, partitions `p0`/`p1`, offsets crescentes, `lag_seconds` ≥ 0.
- Latências são medidas com RAII e enviadas ao histograma `amm_op_latency_seconds` (segundos).

## Logs
- JSON flat com campos obrigatórios (`ts`, `level`, `msg`, `trace_id`, `span_id`, `service`, `env`, `op`, `version`).
- Atributos adicionais ficam em `extra.*` (ex.: `extra.amm.k_before`, `extra.cdc.records`).
- Cada operação gera um log `INFO`. Hooks com falha aparecem como `ERROR` com `error.kind="hook"`.

## Resumo final
O stdout encerra com `obs_demo completed: swap=X pricing=Y cdc=Z`. Processo retorna `0` em sucesso.

## Troubleshooting
- **`failed to bind 127.0.0.1:9464`** → porta em uso. Ajuste `METRICS_HTTP_ADDR`.
- **OTLP ausente em PROD** → configure `OTEL_EXPORTER_OTLP_ENDPOINT`. Sem isso, apenas logs locais.
- **Sem logs/metrics** → verifique `OBSERVABILITY_LEVEL` (não use `off`) e `RUST_LOG`.
- **Processo encerra com erro de configuração** → cheque mensagens do `TelemetryConfig` (regex, booleanos, URL inválida).
