# Sprint 7 — Especificação executiva

## Objetivo canônico
A Sprint 7 consolida o validador único da release Q2 com Gate **T0** obrigatório antes de qualquer etapa de execução.
O foco é garantir:

1. Descoberta determinística dos quatro capítulos normativos da sprint.
2. Execução única e idempotente com geração de evidências auditáveis.
3. Governaça A110 alinhada ao DNA v2 e às lições aprendidas.

## Deliverables principais
- Manifesto NDJSON (`s_7_filemap_v_7.json`) versionado em HEAD com os capítulos obrigatórios.
- Workflow GitHub Actions `.github/workflows/s7-validator.yml` com dois jobs (`t0_spec` e `s7_exec`).
- Artefatos publicados (`s7-t0-evidence`, `s7-orr-evidence`) com resumos e métricas ORR.
- Documentação atualizada (capítulos 2–4) descrevendo contratos, guardrails e fluxo de evidências.

## Fontes de verdade
- DNA v2 (blocos 1–12 e Master List A1–A110).
- Lessons Learned v1 + addendos (Sprint 4) — todos aplicados antes de execução.
- Definition of Awesome Q2/S7.

## Métricas e watchers relevantes
- `data_freshness_seconds`, `drift_score`, `failover_time_p95_s` monitorados pela cadência A110.
- Watchers centrais: `formal_verification_gate_watch`, `metrics_decision_hook_gap_watch`, `dep_vuln_watch`.

## Critérios de aceite (DoD)
- Gate `t0_spec` obrigatório na branch protection e primeiro a ser executado.
- `T0_discovery.json` com `status = PASS`, `checked = 4`, `missing = []`, `sha1_mismatch = []` em HEAD saudável.
- `s7_exec` apenas após `t0_spec = PASS`, validando pins, produzindo bundles e executando checks determinísticos.
- Reexecuções no mesmo commit geram evidências idênticas (exceto IDs/timestamps de infraestrutura).
