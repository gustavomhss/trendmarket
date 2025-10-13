# OBS‑5 — Handoff & Merge Final

## Contexto
- Escopo: Logs JSON + correlação trace_id/span_id (App→OTLP→Collector→Loki)
- Threads concluídas: T1–T6

## Resumo do que este PR consolida
- [x] Artefatos T1 (schema/guia)
- [x] Artefatos T2 (emissor + catálogo op/op_detail)
- [x] Artefatos T3 (Collector logs)
- [x] Artefatos T4 (Loki dev)
- [x] Artefatos T5 (validações + manifesto ORR)
- [x] Artefatos T6 (auditoria de cardinalidade)

## Evidências principais (ORR)
- Manifesto: `out/obs_gatecheck/evidence/logs_pipeline.json`
- Amostras: `schema_sample_500.jsonl`, `pii_sample_1k.txt`
- Auditoria: `cardinality_report.json`, `labels_series_snapshot.json`
- Smoke: `obs5_logs_smoke.txt`

## SLOs (Dev)
- e2e p95 (s): <preencher>
- throughput sustentado (l/s): <preencher>
- parsing fails: 0
- PII matches: 0

## Gates de Aceite (A1–A6)
- [ ] A1 — Campos obrigatórios presentes (≥500 linhas amostradas)
- [ ] A2 — LogQL `| json` projeta `ts/trace_id/msg` corretamente
- [ ] A3 — Labels apenas permitidas (`service,env,op,level` [+`version`])
- [ ] A4 — PII = 0 (amostra 1k)
- [ ] A5 — p95 ≤ 2 s; throughput ≥ 500 l/s (quando testado)
- [ ] A6 — Correlação manual de ao menos 1 `trace_id` no Tempo

## Aprovações
- [ ] EL ✅
- [ ] OBS ✅
- [ ] PM ✅

## Blocos Machine‑Readable
```yaml
ce-orr-obs5-merge:
  commits:
    t1: <sha>
    t2: <sha>
    t3: <sha>
    t4: <sha>
    t5: <sha>
    t6: <sha>
  artifacts:
    manifest: out/obs_gatecheck/evidence/logs_pipeline.json
    samples:
      schema: out/obs_gatecheck/evidence/schema_sample_500.jsonl
      pii: out/obs_gatecheck/evidence/pii_sample_1k.txt
    audit:
      report: out/obs_gatecheck/evidence/cardinality_report.json
      snapshot: out/obs_gatecheck/evidence/labels_series_snapshot.json
    smoke: out/obs_gatecheck/logs/obs5_logs_smoke.txt
  watchers:
    t1: GREEN
    t2: GREEN
    t3: GREEN
    t4: GREEN
    t5: GREEN
    t6: GREEN
  acceptance:
    A1: true
    A2: true
    A3: true
    A4: true
    A5: true
    A6: true
  env:
    loki_endpoint: "http://127.0.0.1:3100"
    collector_health: "http://127.0.0.1:13133/healthz"
```

```

> **Observações:**
> - Preencher `<sha>` com os commits principais de cada thread.
> - Não alterar chaves do bloco YAML — watchers dependem delas.
```
