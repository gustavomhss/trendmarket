# CreditEngine Web (FE)

O diretório `web/` centraliza o frontend Next.js/TypeScript dedicado aos portais e SDKs internos.
Ele segue o SLO de INP p75 ≤ 200 ms e exige cobertura de acessibilidade e CWV.

## Owners & Watchers
- **Owner primário:** Squad FE (Experience Engineering).
- **Watchers ativos:** `web_cwv_regression_watch`, `api_breaking_change_watch`, `dep_vuln_watch`.

## Fluxo operacional
```bash
make lint        # valida inventário, pnpm-lock e README
make test        # garante targets essenciais no Makefile
make build       # gera manifest de build alinhado ao inventário
make evidence    # exporta evidências operacionais para ops/evidence/web.json
```

Os scripts consomem o inventário em `ops/reports/inventory.json` para garantir governança consistente.
