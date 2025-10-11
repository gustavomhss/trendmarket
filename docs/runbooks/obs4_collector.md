# RB-COLLECTOR — Validar e depurar OTel Collector

## Sintomas
- Pipelines de métricas/traces/logs apresentam atraso ou perda.
- Exporters falham silenciosamente ou geram `429/503`.
- Alertas internos apontam gaps de ingestão ou saturação.

## Checagens
- Rodar `otelcol-contrib --config otelcol.yaml --dry-run` (ou `otelcol validate`) para garantir sintaxe/refs válidas.
- Consultar `/metrics` do Collector e observar filas (`otelcol_processor_*`), retry (`otelcol_exporter_*`), limites (`otelcol_receiver_*`).
- Revisar arquivos de erro (`otelcol_trace.err`, `otelcol_metrics.err`, `otelcol_logs.err`) e logs STDOUT.
- Confirmar variáveis de ambiente críticas: `OTEL_RESOURCE_ATTRIBUTES`, credenciais de exporters, endpoints (`OTEL_EXPORTER_OTLP_*`).
- Validar portas expostas por pipeline: receivers OTLP gRPC/4317, HTTP/4318, Zipkin/9411, Jaeger/14250, Prometheus/9464 etc.
- Checar limites de batch/queue (`batch::send_batch_size`, `queue_size`) versus throughput atual.

## Ações
1. Executar `otelcol-contrib --config otelcol.yaml --dry-run` após qualquer mudança e corrigir erros apontados.
2. Habilitar log em nível `debug` temporariamente (`service.telemetry.logs.level`) para reproduzir e capturar falha.
3. Inspecionar métricas `/metrics` com `curl localhost:9464/metrics | grep otelcol_` e identificar gargalos.
4. Ajustar parâmetros de batch/queue ou adicionar processadores de retry conforme necessidade.
5. Validar comunicação com destinos (Tempo, Loki, Prometheus) usando `nc`/`openssl s_client` nas portas listadas.
6. Consolidar evidências e reverter logging a `info` após estabilizar.

## Artefatos
- Saída do `--dry-run`/`validate` mostrando sucesso ou erros corrigidos.
- Captura de métricas `/metrics` destacando filas/drops antes/depois.
- Logs `otelcol_*.err` com timestamp e correção aplicada.
- Tabela de portas/variáveis relevantes anexada ao incidente.
