# OBS-5 ORR Merge Checklist (T7)

> Objetivo: confirmar que as evidências e watchers das threads T1–T6 sustentam o merge final sem regressões operacionais.

## 1. Pré-checks
- [ ] Confirmar que o branch ativo é `obs5/t7-handoff-pr`.
- [ ] `git status` limpo exceto pelos artefatos deste T7.
- [ ] Links de referência disponíveis:
  - Manifesto ORR: `out/obs_gatecheck/evidence/logs_pipeline.json`
  - Samples: `out/obs_gatecheck/evidence/schema_sample_500.jsonl` e `out/obs_gatecheck/evidence/pii_sample_1k.txt`
  - Auditoria: `out/obs_gatecheck/evidence/cardinality_report.json` e `out/obs_gatecheck/evidence/labels_series_snapshot.json`
  - Smoke: `out/obs_gatecheck/logs/obs5_logs_smoke.txt`

## 2. Revisão humana
- [ ] Validar que o template de PR `.github/pull_request_template_obs5_t7.md` carrega com todos os checkboxes.
- [ ] Verificar que o bloco `ce-orr-obs5-merge` está preenchido com os SHAs das threads T1–T6.
- [ ] Confirmar que os campos de SLO p95 e throughput foram preenchidos com os resultados finais.
- [ ] Revisar notas de `ops/reports/` para garantir que não houve waivers pendentes.

## 3. Watchers e evidências
- [ ] Executar `make watchers.dry` e verificar `watcher.obs5.merge.*` GREEN.
- [ ] Conferir `out/obs_gatecheck/evidence/merge_manifest.json` para `verdict.status == "ready"`.
- [ ] Validar que os paths referenciados no manifesto existem no repositório.

## 4. Cross-check Tempo ↔ Loki
1. Identificar um `trace_id` presente nas amostras (`schema_sample_500.jsonl`).
2. **Tempo:** abrir Grafana Tempo com o modelo abaixo substituindo `<trace_id>`:
   - `https://grafana.local/tempo/explore?left={"range":{"from":"now-1h","to":"now"},"queries":[{"datasource":{"type":"tempo","uid":"tempo"},"query":"{trace_id='<trace_id>'}"}]}`
3. **Loki:** usar LogQL `| json trace_id="<trace_id>"` e confirmar correlação com `span_id` e `service`.
4. Capturar evidência (screenshot ou export) anexando ao review.

## 5. Aprovações obrigatórias
- [ ] EL ✅
- [ ] OBS ✅
- [ ] PM ✅

> Conclua o merge apenas com todas as caixas marcadas e watchers GREEN.
