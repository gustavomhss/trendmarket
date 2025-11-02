# Sprint 7 — Codex Harness & Guardrails

## Responsabilidades do Codex
- Gerar e versionar `s_7_filemap_v_7.json` com hashing `git hash-object`.
- Implementar o validador linha-a-linha em `scripts/s7_validator.py`, evitando parsers frágeis.
- Garantir que o workflow `s7-validator.yml` seja idempotente, determinístico e com `concurrency` por ref.
- Validar pins de actions contra `actions.lock`, impedindo refs soltas.

## Processo automatizado
1. **Geração do manifesto** — `python scripts/s7_validator.py generate-manifest`.
2. **Gate T0** — `python scripts/s7_validator.py validate-manifest --output out/obs_gatecheck/T0_discovery.json`.
3. **Download/Upload de evidências** — artifacts `s7-t0-evidence` e `s7-orr-evidence`.
4. **Validação de pins** — `python scripts/s7_validator.py validate-actions --workflow .github/workflows/s7-validator.yml`.
5. **Bundle determinístico** — `python scripts/s7_validator.py bundle-evidence` gerando `out/s7-orr-evidence.zip` e `RESUMO_ORR_S7.json`.

## Guardrails críticos
- Sem dependência de rede externa para validação (apenas ferramentas pré-instaladas).
- Seeds fixos (`PYTHONHASHSEED=0`, `CI_SEED=12345`) para testes e coletores.
- Logs objetivos com anotação GitHub (`::error`, `::warning`).
- Artifacts sempre publicados mesmo em falha (`if: always()`).

## Estrutura do script `s7_validator.py`
- `CHAPTERS`: lista canônica dos quatro paths.
- `generate_manifest()` → escreve NDJSON com uma linha por capítulo.
- `validate_manifest()` → lê NDJSON linha a linha, valida existência/sha1 e produz JSON.
- `validate_actions()` → compara refs declaradas com `actions.lock` e falha em divergências.
- `capture_versions()` → salva versões de ferramentas em JSON.
- `bundle_evidence()` → monta filelist ordenado e zip determinístico com watchers/métricas no `RESUMO_ORR_S7.json`.

## Métricas registradas
- `data_freshness_seconds`, `drift_score`, `failover_time_p95_s`.
- Veredito consolidado dos gates (`T0`, `S7_EXEC`).
- Commit e branch ativos, além das versões de ferramentas utilizadas.

## Execução local
```bash
python scripts/s7_validator.py generate-manifest
python scripts/s7_validator.py validate-manifest --output out/obs_gatecheck/T0_discovery.json
python scripts/s7_validator.py validate-actions --workflow .github/workflows/s7-validator.yml
python scripts/s7_validator.py capture-versions --output out/orr_s7/tool_versions.json
python scripts/s7_validator.py bundle-evidence --resumo out/orr_s7/RESUMO_ORR_S7.json --filelist out/orr_s7/filelist.txt --zip out/s7-orr-evidence.zip
```

## Checklist de robustez
- [x] Manifesto sincronizado com HEAD após cada alteração nos capítulos.
- [x] Validadores sem parsing heurístico.
- [x] Actions travadas por commit com auditoria via `actions.lock`.
- [x] Evidências versionadas e reutilizáveis para auditoria ORR/Watchers A110.
