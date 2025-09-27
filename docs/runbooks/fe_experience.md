# Runbook FE — Experiência & SDKs

## web-cwv — `web_cwv_regression_watch`
- **Hook:** `web-cwv-rollback`
- **KPI:** `web.cwv.inp_p75` ≤ 200 ms (janela 24h)
- **Ação automática:** `rollback_fe_release`
- **Owner:** FE
- **Rollback:** após INP ≤ 180 ms por 2 janelas.

**Passos**
1. Validar deploy recente (Next.js) e métricas de Web Vitals.
2. Checar monitoramento RUM e comparar com canário.
3. Aplique rollback via pipeline FE e comunique squads consumidores.

## api-breaking — `api_breaking_change_watch`
- **Hook:** `api-contract-block`
- **KPI:** `api.contract.breaking_changes` = 0 (janela 5m)
- **Ação automática:** `block_release`
- **Owner:** INT + FE
- **Rollback:** após publicação de contrato compatível e reexecução de testes.

**Passos**
1. Rodar suite de contract tests (`make be.contracts`).
2. Verificar se houve alteração em SDK/FE e backend correspondente.
3. Atualizar changelog e sinalizar nova versão do contrato.
