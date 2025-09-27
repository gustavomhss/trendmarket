# CreditEngine Data (CDC & Lakehouse)

O diretório `data/` guarda pipelines CDC→Iceberg, contratos A106/A87/A89 e materialização dbt.
Ele sustenta o SLA de lag p95 ≤ 120 s e monitora compatibilidade no schema registry.

## Owners & Watchers
- **Owner primário:** Squad DATA (Data Platform Chapter).
- **Watchers ativos:** `data_contract_break_watch`, `schema_registry_drift_watch`, `cdc_lag_watch`, `dbt_test_failure_watch`.

## Fluxo operacional
```bash
make lint        # valida README, inventário e lockfile do domínio
make test        # confirma alvos essenciais no Makefile
make build       # materializa manifestos determinísticos
make evidence    # exporta evidências em ops/evidence/data.json
```
