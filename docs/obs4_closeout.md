# OBS-4 Closeout Checklist

## Pré-gate
- [ ] Confirmar que todos os threads (01–09) foram executados com evidência anexada.
- [ ] Validar que `ops/otel/collector-dev.trace.yaml` reflete a configuração vigente do collector de traces.
- [ ] Revisar watchers e hooks A110 para o pack OBS-4 com owners e plano de rollback.

## Evidências e artefatos
- [ ] Executar `scripts/obs4_finalize.sh` para gerar comentário Jira, ZIP e branch dedicado.
- [ ] Verificar se o ZIP contém `ops/otel/collector-dev.trace.yaml`, scripts `obs4*`, docs `obs4_*` e pastas de evidência/logs.
- [ ] Garantir que `traces_sample.json`, `traces_raw.json` e `obs4_trace_smoke.txt` estejam atualizados e armazenados em `out/obs_gatecheck`.

## Comentário Jira
- [ ] Checar se as versões das ferramentas e status dos gates estão no comentário.
- [ ] Incluir resumo dos spans canônicos e políticas de tail sampling.
- [ ] Anexar links/paths das evidências principais.

## Git e PR
- [ ] Confirmar que o script criou branch `obs4/closeout-<timestamp>` (quando acionado a partir de `main`).
- [ ] Validar commit `obs(OBS-4): tracing + evidências ORR` com todos os artefatos relevantes.
- [ ] Conferir push para `origin` e abertura do PR contra `main` (ou executar manualmente se `gh` indisponível).

## Pós-gate
- [ ] Registrar no Jira o comentário gerado e anexar o ZIP de closeout.
- [ ] Garantir que as evidências fiquem arquivadas em storage corporativo após merge.
- [ ] Atualizar runbook de observabilidade caso novas lições tenham sido aprendidas.
