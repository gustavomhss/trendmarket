# OBS-1 • `obs_demo` Binary Contract

## 1. Propósito
O binário `obs_demo` demonstra, de ponta a ponta, a instrumentação OBS-1 da aplicação CreditEngine. Ele inicializa tracing, métricas e logs JSON correlacionados com os contratos aprovados, gera tráfego sintético coerente (AMM, pricing e CDC) e encerra fornecendo um resumo da execução. Este documento é normativo para operadores e integrações automáticas.

## 2. Variáveis de ambiente

| Variável | Obrigatória | Default | Descrição |
| --- | --- | --- | --- |
| `DEPLOY_ENV` | Não | `dev` | Ambiente lógico (`dev`, `stg`, `prod`). Propaga para a identidade e labels. |
| `OBSERVABILITY_LEVEL` | Não | `min` | Volume de telemetria (`off`, `min`, `full`). `off` evita exportadores OTLP e métrica Prometheus. |
| `PROM_SCRAPE` | Não | `off` | Quando `on`, abre `/metrics` no endereço definido em `METRICS_HTTP_ADDR`. |
| `METRICS_HTTP_ADDR` | Não | `0.0.0.0:9464` | Endereço de escuta do servidor Prometheus de desenvolvimento. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | Condicional | — | Endpoint OTLP HTTP (ex.: `http://otel-collector:4318/v1/metrics`). Obrigatório em produção. |
| `LOG_LEVEL` | Não | `info` | Nível de log estruturado (`trace|debug|info|warn|error`). |
| `SERVICE_NAME` | Não | `ce-amm` | Nome lógico do serviço. Respeita regex `^[a-z0-9._-]{3,64}$`. |
| `SERVICE_VERSION` | Não | `0.0.0+<git>` | Versão reportada nos recursos. Pode ser sobreposta para experimentos. |
| `OBS_DEMO_OPS` | Não | `20` | Número de operações sintéticas executadas antes do término. |

## 3. Inicialização e identidade
1. Carrega `TelemetryConfig` conforme contrato T2 (precedência builder → env → default) e valida valores.
2. Resolve `ServiceIdentity` (T3) incluindo `service.name`, `service.version`, `deployment.environment`, `build.time.utc` e `git.sha` provenientes do `build.rs`.
3. Configura tracer provider (`tracing_opentelemetry`) com recursos canonizados. Quando `OTEL_EXPORTER_OTLP_ENDPOINT` está presente e `OBSERVABILITY_LEVEL` ≠ `off`, habilita exportação OTLP/HTTP.
4. Para métricas, registra `amm_op_latency_seconds` e `hook_executions_total` com os buckets e labels definidos em `telemetry_contract.rs`. OTLP é habilitado somente quando endpoint presente. No modo dev (`PROM_SCRAPE=on`), um servidor HTTP expõe `/metrics` com as séries em Prometheus text format.
5. Instala o formatter JSON de logs (T7) garantindo campos obrigatórios (`ts`, `level`, `msg`, `trace_id`, `span_id`, `service`, `env`, `op`, `version`) e inserindo payloads adicionais em `extra` de forma hierárquica (ex.: `extra.amm.k_before`).

## 4. Workloads sintéticas
O loop principal alterna entre três operações, respeitando `OBS_DEMO_OPS`:

### 4.1 AMM — `amm.swap`
- Simula um pool CPMM com reservas iniciais `1_200 × 800` ajustadas por um fator de liquidez.
- Escolhe direção (`base_to_quote` ou `quote_to_base`), `fee_ppm ∈ [100, 500]`, entrada `10..120`. O fee é aplicado antes do cálculo.
- Gera `ε ∈ [-0.05, 0.05]`, recalibrando o invariante: `k_after = k_before * (1+ε)`.
- Span `amm.swap` registra atributos `amm.k_before`, `amm.k_after`, `amm.delta_k_ratio`, `amm.fee_ppm`, `amm.input`, `amm.output`, `amm.direction` e os recursos `service.*` e `deployment.environment`.
- Um hook sintético `amm.risk-check` é avaliado em cada swap e incrementa `hook_executions_total{hook_id="amm.risk-check",status="success|error"}` (95% sucesso, 5% erro).

### 4.2 Pricing — `pricing.quote`
- Calcula cotações hipotéticas respeitando o mesmo invariante e spread (`ε ∈ [-0.02, 0.02]`).
- Não altera reservas base, apenas atualiza o fator de liquidez.
- Span `pricing.quote` replica os atributos do contrato e adiciona `pricing.mid_price`.

### 4.3 CDC — `cdc.consume`
- Alterna streams `trades`/`quotes` e partitions `p0`/`p1`.
- Atualiza offsets cumulativos (`offset_after = offset_before + records`, com `records ∈ [1, 50]`).
- Gera `lag_seconds ∈ [0, 5]` coerente com ingestão controlada.
- Span `cdc.consume` inclui atributos `cdc.stream`, `cdc.partition`, `cdc.offset_before`, `cdc.offset_after`, `cdc.records`, `cdc.lag_seconds`, além dos campos AMM para compatibilidade.

Cada operação mede latência com RAII (`LatencyGuard`) e registra o valor em segundos no histograma `amm_op_latency_seconds{op,service,env,version}`.

## 5. Logs estruturados
- Emissão `INFO` para cada operação com `msg` contextual (`swap executed`, `pricing quote generated`, `cdc batch consumed`).
- Campos adicionais são serializados em `extra` com hierarquia (`extra.amm.fee_ppm`, `extra.cdc.records`).
- `trace_id`/`span_id` são extraídos do `tracing_opentelemetry::OpenTelemetrySpanExt`, garantindo 32/16 caracteres hexadecimais.
- Erros simulados (hook falho) aparecem como eventos separados com `error.kind="hook"` e `error.message` correspondente.

## 6. Métricas `/metrics`
Quando `PROM_SCRAPE=on`, o binário imprime `Prometheus exporter listening at http://<addr>/metrics` e inicia um servidor assíncrono. O payload inclui:

```
# HELP amm_op_latency_seconds Latency per AMM operation in seconds
# TYPE amm_op_latency_seconds histogram
amm_op_latency_seconds_bucket{op="swap",service="ce-amm",env="dev",version="0.1.0",le="0.050000"} 3
...
amn_op_latency_seconds_sum{...} 0.842
amm_op_latency_seconds_count{...} 5
# HELP hook_executions_total Hook executions by hook_id and status
# TYPE hook_executions_total counter
hook_executions_total{hook_id="amm.risk-check",status="success"} 18
```

Os buckets seguem `telemetry_contract::AMM_OP_LATENCY_BUCKETS` e são cumulativos, com linha adicional `+Inf`, `_sum` e `_count`.

## 7. Modos de operação

### 7.1 DEV (Prometheus)
```
export DEPLOY_ENV=dev
export OBSERVABILITY_LEVEL=full
export PROM_SCRAPE=on
export METRICS_HTTP_ADDR=127.0.0.1:9464
RUST_LOG=info cargo run --bin obs_demo
```
Resultado esperado: logs JSON no stdout, mensagem de startup do servidor `/metrics` e sumário final.

### 7.2 STG — OTLP HTTP 4318
```
export DEPLOY_ENV=stg
export OBSERVABILITY_LEVEL=min
export OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector.stg:4318/v1/metrics
RUST_LOG=info cargo run --bin obs_demo
```
Resultado esperado: spans e métricas enviados via OTLP, logs JSON no stdout e ausência do servidor `/metrics` (a menos que `PROM_SCRAPE=on`).

### 7.3 PROD
- `OBSERVABILITY_LEVEL` deve ser `min` ou `full`.
- `OTEL_EXPORTER_OTLP_ENDPOINT` obrigatório (`https://...` recomendado).
- `PROM_SCRAPE` deve permanecer `off`, exceto para debug controlado.
- Qualquer falha de validação na configuração aborta o processo com mensagem explícita.

## 8. Encerramento e códigos de saída
- Após `OBS_DEMO_OPS` operações, o binário imprime `obs_demo completed: swap=X pricing=Y cdc=Z` e retorna código `0`.
- Erros de inicialização (endereço inválido, OTLP malformado, env proibida) resultam em `stderr` detalhado e `exit(1)` via `anyhow`.

## 9. Garantias de não-placeholder
- Valores numéricos derivam de um gerador determinístico com ranges realistas.
- `k_after` é sempre `k_before*(1+ε)` com `ε` dentro dos limites especificados.
- Offsets CDC crescem monotonicamente e `lag_seconds` é não-negativo.
- Hooks variam entre sucesso/erro com distribuição fixa, alimentando o counter canônico.
- Logs trazem `extra` coerente com os atributos dos spans.

## 10. Troubleshooting
- **Porta ocupada no `/metrics`**: erro `failed to bind 127.0.0.1:9464` durante bootstrap. Ajuste `METRICS_HTTP_ADDR`.
- **Collector ausente**: warning `OTLP endpoint missing` será emitido quando `level != off`. Execução continua com telemetria local.
- **Env inválido**: mensagens de `TelemetryConfig::from_env` orientam correção (regex violado, booleano inválido etc.).
- **Execução sem spans/logs**: confirme `OBSERVABILITY_LEVEL` diferente de `off` e `RUST_LOG` compatível.

## 11. Referências cruzadas
- `src/bin/obs_demo.rs`: implementação oficial.
- `tests/obs_demo_smoke_tests.rs`: validações automatizadas (`cargo test`).
- `out/obs_gatecheck/evidence/obs1_obs_demo_report.json`: evidência operacional (timestamp, logs, métricas, hash SHA256).
