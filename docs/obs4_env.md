# OBS-4 Trace Collector (Dev)

Este guia resume como validar e operar o pipeline de traces configurado para o Collector local nesta thread.

## Comandos principais

```bash
# valida a configuração YAML
bash scripts/obs4_collector_trace.sh validate

# inicia o Collector em background
bash scripts/obs4_collector_trace.sh start

# verifica processo e endpoint de métricas (/metrics)
bash scripts/obs4_collector_trace.sh status

# encerra o Collector
bash scripts/obs4_collector_trace.sh stop
```

Todos os artefatos de execução (stdout, stderr, PID, dumps) são gravados em `out/obs_gatecheck/logs/`.

## Variáveis de ambiente relevantes

| Variável | Função | Default |
| --- | --- | --- |
| `TAIL_SLOW_MS` | Limite de latência (ms) usado na política de tail sampling para spans com prefixo `amm.`/`pricing.` do serviço alvo. | `200` |
| `TEMPO_OTLP_HTTP` | Endpoint OTLP/HTTP do Tempo (traces). Caso esteja definido, o pipeline exporta para `/v1/traces`. | `http://localhost:4318` |
| `JAEGER_OTLP_HTTP` | Endpoint OTLP/HTTP do Jaeger (traces). Usado como fallback/alternativa em paralelo ao Tempo. | `http://localhost:4318` |

Outras variáveis opcionais incluem `OTLP_GRPC_PORT`, `OTLP_HTTP_PORT`, `OTELCOL_LISTEN_ADDR` e `OTELCOL_LISTEN_PORT`, que controlam os endpoints de recepção e a telemetria interna do Collector.
