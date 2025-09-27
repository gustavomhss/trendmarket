# CreditEngine Infra & Observabilidade

O diretório `infra/` agrega IaC, provisionamento de observabilidade e pipelines de entrega.
Ele garante que SLO/SLI, alertas e políticas de segurança estejam versionados com owners claros.

## Owners & Watchers
- **Owner primário:** Squad SRE/Plataforma.
- **Watchers ativos:** `slo_budget_breach_watch`, `runtime_eol_watch`, `dep_vuln_watch`, `alert_storm_watch`.

## Fluxo operacional
```bash
make lint        # valida inventário, README e terraform.lock
make test        # garante targets essenciais no Makefile
make build       # gera plano sintético (plan.txt)
make evidence    # sincroniza evidências em ops/evidence/infra.json
```
