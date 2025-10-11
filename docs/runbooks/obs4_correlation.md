# RB-CORRELATION — Logs sem `trace_id`

## Sintomas
- Busca por `trace_id` no Loki não retorna eventos apesar de existir trace correspondente.
- Alertas de correlação falham ao montar timeline end-to-end.

## Checagens
- Garantir que a camada `tracing-subscriber` está emitindo JSON com campos `trace_id` e `span_id` (ver config/feature flags).
- Inspecionar amostras recentes no Loki (`{app="core"} |= "trace_id"`) para confirmar presença e formato consistente.
- Validar pipeline de labels do Loki: transformar `trace_id` em label (`label_keys`, `pipeline_stages`) e índice habilitado.
- Confirmar timezone usado nas queries e dashboards: deslocamentos ocultam eventos aparentemente ausentes.
- Revisar ingestão no Collector (processador de logs) para garantir que atributos OTel estão sendo propagados.

## Ações
1. Executar queries de exemplo: `{trace_id="<id>"}` e `{app="core", trace_id=~".+"}` para testar índice e cardinalidade.
2. Ajustar a configuração do `tracing-subscriber` para serializar contextos (ex.: `with_span_events`, `fmt::layer().json()` com campos).
3. Atualizar pipeline de transformação no Loki (`match`, `json`, `labels`) adicionando `trace_id`/`span_id` quando ausentes.
4. Reindexar/compactar tabela alvo se necessário (`loki-canary`, `compactor`) para refletir novas labels.
5. Verificar timezone nas consultas (`| unwrap ts | line_format "{{ .ts | toTimeZone \"UTC\" }}"`) e alinhar dashboards.

## Artefatos
- Query Loki + resultado (captura ou JSON) mostrando falha inicial e sucesso após correção.
- Diff de configuração (`tracing-subscriber`, pipeline Loki) aplicado.
- Trace/log pair (trace_id, span_id) confirmando correlação restaurada.
