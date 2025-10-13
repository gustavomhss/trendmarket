# Tempo Link Cookbook

Use os modelos abaixo para criar links diretos no Grafana Tempo com base em `trace_id`.

## 1. Link padrão (Explore)
```
https://grafana.local/tempo/explore?left={"range":{"from":"now-1h","to":"now"},"queries":[{"refId":"A","datasource":{"type":"tempo","uid":"tempo"},"query":"{trace_id='${TRACE_ID}'}"}]}
```
- Substitua `${TRACE_ID}` pelo valor desejado.
- Ajuste a janela (`from`/`to`) conforme o horário do trace.

## 2. Link com foco em span específico
```
https://grafana.local/tempo/explore?left={"range":{"from":"now-2h","to":"now"},"queries":[{"refId":"A","datasource":{"type":"tempo","uid":"tempo"},"query":"{trace_id='${TRACE_ID}'} |= ${SPAN}`}]}
```
- `${SPAN}` deve ser um identificador único do span (`service="collector"`, por exemplo).
- Útil para validar ingestão Collector→Loki.

## 3. Link usando parâmetros amigáveis
```
https://grafana.local/tempo/explore?left=%7B"range"%3A%7B"from"%3A"now-30m"%2C"to"%3A"now"%7D%2C"queries"%3A%5B%7B"refId"%3A"A"%2C"datasource"%3A%7B"type"%3A"tempo"%2C"uid"%3A"tempo"%7D%2C"query"%3A"%7Btrace_id%3D'%24%7BTRACE_ID%7D'%7D"%7D%5D%7D
```
- Variante URL-encoded para colar em runbooks.
- Mantém compatibilidade com markdown/Slack.

> Combine estes links com as consultas Loki `| json trace_id="${TRACE_ID}"` para validar correlação ponta a ponta.
