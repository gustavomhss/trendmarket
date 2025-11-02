# Sprint 7 — Gates, ORR e Scorecards

## Gate T0 — Spec Discovery (único e obrigatório)
O Gate **T0** executa como primeiro job do workflow oficial `s7-validator.yml`.
Ele garante a existência e a integridade criptográfica dos quatro capítulos canônicos listados abaixo.

### Manifesto NDJSON (`s_7_filemap_v_7.json`)
- Formato NDJSON com **uma linha por arquivo**.
- Estrutura exata: `{ "path": "<relativo ao repo>", "bytes": <int>, "sha1": "<git hash-object>" }`.
- Lista **exclusivamente** os arquivos:
  1. `docs/DNA/Roadmap e Sprints/Q2/Sprint 7/s_7_capitulo_1_spec_v_7.md`
  2. `docs/DNA/Roadmap e Sprints/Q2/Sprint 7/s_7_capitulo_2_gates_orr_scorecards_v_1.md`
  3. `docs/DNA/Roadmap e Sprints/Q2/Sprint 7/s_7_capitulo_3_filemap_100_v_1.md`
  4. `docs/DNA/Roadmap e Sprints/Q2/Sprint 7/s_7_capitulo_4_codex_harness_guardrails_v_1.md`

### Contrato do artefato `T0_discovery.json`
O job gera `out/obs_gatecheck/T0_discovery.json` com o contrato rígido:
```json
{
  "gate": "T0",
  "status": "PASS" | "FAIL",
  "checked": 4,
  "missing": ["<paths>"] ,
  "sha1_mismatch": ["<paths>"]
}
```
- `status = PASS` **apenas** quando `missing = []`, `sha1_mismatch = []` e `checked = 4`.
- `missing` agrega tanto arquivos ausentes quanto entradas faltantes no manifesto.
- `sha1_mismatch` captura qualquer divergência de conteúdo ou entrada inesperada.
- Erros são logados via `::error` e divergências via `::warning`.

### Exemplos canônicos
**Passo feliz**
```json
{
  "gate": "T0",
  "status": "PASS",
  "checked": 4,
  "missing": [],
  "sha1_mismatch": []
}
```

**Falha por arquivo ausente**
```json
{
  "gate": "T0",
  "status": "FAIL",
  "checked": 3,
  "missing": ["docs/DNA/Roadmap e Sprints/Q2/Sprint 7/s_7_capitulo_4_codex_harness_guardrails_v_1.md"],
  "sha1_mismatch": []
}
```

**Falha por drift de conteúdo**
```json
{
  "gate": "T0",
  "status": "FAIL",
  "checked": 4,
  "missing": [],
  "sha1_mismatch": ["docs/DNA/Roadmap e Sprints/Q2/Sprint 7/s_7_capitulo_2_gates_orr_scorecards_v_1.md"]
}
```

## Gate S7 Exec — ORR integrado
O job `s7_exec` executa apenas quando `t0_spec = PASS` e produz o bundle oficial de evidências.

### Etapas e watchers
1. **Rastreabilidade de ferramentas** — versões de `python`, `jq`, `yamllint`.
2. **Scripts de CI** — render/validate quando presentes (`[skip]` se ausentes).
3. **Linting YAML** — não falha se `yamllint` indisponível.
4. **Gitleaks (detecção)** — gera `out/evidence/T2_security/gitleaks_report.json` com status dos watchers `dep_vuln_watch` e `security_supply_chain_watch`.
5. **Testes determinísticos** — `PYTHONHASHSEED=0`, `CI_SEED=12345` e log associado aos watchers `formal_verification_gate_watch` e `metrics_decision_hook_gap_watch`.
6. **Microbench & TLA** — opcional, sempre registrando `[skip]` quando não aplicável.
7. **Validação de pins** — confronta `s7-validator.yml` com `actions.lock`; divergências encerram o job (`actions.lock mismatch`).
8. **Bundle ORR** — gera `RESUMO_ORR_S7.json`, `out/orr_s7/filelist.txt` e `out/s7-orr-evidence.zip` (determinístico) referenciando watchers `data_freshness_watch`, `drift_score_watch` e `failover_time_p95_watch`.

### Artefatos obrigatórios
- `s7-t0-evidence` → `out/obs_gatecheck/T0_discovery.json`.
- `s7-orr-evidence` → `out/s7-orr-evidence.zip`, `out/orr_s7/filelist.txt`, `out/orr_s7/RESUMO_ORR_S7.json` e relatórios auxiliares.

## Scorecard de auditoria
- **T0**: 100% de cobertura nos capítulos críticos.
- **Segurança**: zero segredos expostos, relatório gitleaks versionado.
- **Reprodutibilidade**: reexecuções no mesmo commit com filelist idêntico.
- **Governança**: branch protection exige `t0_spec` e `s7_exec` verdes.
