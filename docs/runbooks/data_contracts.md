# Runbook DATA — Contratos & Pipelines

## cdc-lag — `cdc_lag_watch`
- **Hook:** `data-contract-rollback`
- **KPI:** `cdc.lag.p95` ≤ 120 s (janela 15m)
- **Ação automática:** `degrade_to_hot_table`
- **Owner:** DATA
- **Rollback:** quando lag ≤ 90 s por 3 janelas.

**Checklist**
1. Verificar DLQ e conectividade Debezium → Iceberg.
2. Avaliar backlog de offsets; acionar `scripts/cdc/replay.sh` se necessário.
3. Coordenar com DEC sobre consumo de hot tables e avisar stakeholders.
4. Registrar incident no `data-contracts` board com evidências.

## schema-drift — `schema_registry_drift_watch`
- **Hook:** `data-contract-rollback`
- **KPI:** ausência de `schema.drift_detected` (janela 5m)
- **Ação automática:** `block_deploy`
- **Owner:** DATA
- **Rollback:** após publicação de schema compatível e testes de contrato.

**Checklist**
1. Rodar `scripts/schema/compare.sh` para diff detalhado.
2. Validar compatibilidade backward e publicar novo contrato no registry.
3. Atualizar consumidores afetados e obter aprovação A87/A89.

## dbt-tests — `dbt_test_failure_watch`
- **Hook:** `data-contract-rollback`
- **KPI:** `dbt.tests.failure_rate` = 0 (janela 15m)
- **Ação automática:** `rollback_transform`
- **Owner:** DATA
- **Rollback:** após rerun `dbt test` com sucesso.

**Checklist**
1. Executar `make data.dbt` local para reproduzir.
2. Conferir alterações recentes de modelos e seeds.
3. Aplicar hotfix ou rollback da run e comunicar release train.

## contract-breach — `data_contract_break_watch`
- **Hook:** `data-contract-rollback`
- **KPI:** `data.contract.break` = 0 (janela 5m)
- **Ação automática:** `trigger_contract_waiver`
- **Owner:** DATA
- **Rollback:** após waiver aprovado e contrato restaurado.

**Checklist**
1. Validar métricas de completude/freshness.
2. Avaliar impacto em DEC/ML; se crítico, acionar degrade imediato.
3. Atualizar contrato com versão corrigida e auditar no ACE.

## doc-coverage — `doc_coverage_watch`
- **Hook:** `data-contract-rollback`
- **KPI:** `data.doc.coverage` ≥ 95% (janela 24h)
- **Ação:** `raise_doc_gap`
- **Owner:** DATA
- **Rollback:** não aplicável (processo contínuo).

**Checklist**
1. Gerar relatório de cobertura via `scripts/data/doc_coverage.py`.
2. Priorizar gaps no board `DATA-TechDebt`.
3. Incluir plano de ação no próximo weekly review.
