# CreditEngine Backend (DEC)

O diretório `be/` consolida o runtime FastAPI/FastStream responsável pela orquestração DEC.
Ele está alinhado ao SLO de decisão p95 ≤ 800 ms e mantém contratos auditáveis.

## Owners & Watchers
- **Owner primário:** Squad DEC (Decision Engineering Chapter).
- **Watchers ativos:** `api_breaking_change_watch`, `metrics_decision_hook_gap_watch`, `slo_budget_breach_watch`, `runtime_eol_watch`, `dep_vuln_watch`.

## Fluxo operacional
Use os alvos de `Makefile` para garantir consistência local:

```bash
make lint        # valida inventário, estilo e dependências fixadas
make test        # garante aderência de watchers/contratos ao inventário
make build       # materializa artefatos de build (manifest.json)
make evidence    # publica evidências em ops/evidence/be.json
make hooks.dry   # valida gramática mínima de hooks A110 para o domínio
make watchers.dry# confirma cobertura dos watchers obrigatórios
```

Os scripts dependem do inventário central em `ops/reports/inventory.json`.
Qualquer alteração de owner/watchers deve atualizar esse inventário.
