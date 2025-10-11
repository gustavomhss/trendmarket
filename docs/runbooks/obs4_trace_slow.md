# RB-TRACE-SLOW — Span lento não aparece

## Sintomas
- Span conhecido como lento deixa de surgir em buscas na UI/Tempo/Jaeger mesmo sob carga.
- Dashboards tail latency planos ou zerados enquanto usuários reportam lentidão.

## Checagens
- Confirmar `TAIL_SLOW_MS` na config do Collector/Tempo: valor compatível com o SLA do span.
- Verificar fila `tail_sampling` via métricas `otelcol_processor_batch_queue_size` ou equivalente: sem backlog ou drops.
- Conferir `decision_wait` do tail sampling: comparar janela com a duração esperada do span.
- Observar tempo de flush do batch (`otelcol_processor_batch_batch_send_size`, `send_latency`): filas grandes indicam atraso.
- Validar relógio do host (`chronyc tracking`/`ntpq -p`): desvios >100ms podem descasar janelas de decisão.
- Inspecionar sampler no SDK (probabilidade e filtros); garantir que spans de interesse não são descartados antes do Collector.

## Ações
1. Aumentar a janela de decisão (`decision_wait`/`decision_wait_time`) em pequenos incrementos (ex.: +1s) e revalidar ingestão.
2. Reduzir `sampling_ratio` no tail sampler para aliviar pressão temporária, mantendo cobertura mínima acordada.
3. Injetar tráfego controlado com o script smoke (`scripts/obs/trace_tail_smoke.sh` ou equivalente) para reproduzir spans lentos.
4. Coletar `otelcol_trace.err` e logs do pipeline de sampling; anexar se o span continuar invisível.
5. Reverter ajustes para valores finais aprovados após estabilizar a captura.

## Artefatos
- `otelcol_trace.err` e trechos de log com parâmetros de sampling aplicados.
- Capturas de métricas (`/metrics`) evidenciando filas antes/depois da intervenção.
- Trace IDs gerados pelo smoke test demonstrando presença/ausência do span lento.
