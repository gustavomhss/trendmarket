# Changeset — Domain topology refresh

## Resumo
- Estrutura criada para os domínios `be/`, `web/`, `data/`, `ml/`, `specs/`, `infra/` e `ops/tests` com README, Makefile e scripts de bootstrap obedecendo ao contrato de lint/test/build/evidence.
- Locks dedicados adicionados (`uv.lock`, `pnpm-lock.yaml`, `terraform.lock.hcl`) garantindo reprodutibilidade mínima por domínio.
- Inventário operacional (`ops/reports/inventory.json`) atualizado com owners, watchers e artefatos governados.
- Evidências padronizadas configuradas para publicação em `ops/evidence/*.json` a partir dos scripts locais.

## Impacto em SLO/SLA
- Nenhum impacto direto em produção; habilita governança e execução consistente dos comandos canônicos.

## Watchers tocados
- `api_breaking_change_watch`, `metrics_decision_hook_gap_watch`, `web_cwv_regression_watch`, `data_contract_break_watch`, `schema_registry_drift_watch`, `cdc_lag_watch`, `dbt_test_failure_watch`, `model_drift_watch`, `ab_srm_watch`, `slo_budget_breach_watch`, `runtime_eol_watch`, `dep_vuln_watch`, `formal_verification_gate_watch`, `okr_risk_alignment_watch`, `policy_violation_watch`, `alert_storm_watch`.

## Evidências
- Manifestos de domínio em `<domain>/build/manifest.json`.
- Arquivos `ops/evidence/<domain>.json` publicados via `make evidence`.
- Lockfiles revisados e versionados no repositório.
