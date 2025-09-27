# CreditEngine Specs & Governança

O diretório `specs/` consolida ADRs, PR-FAQ, hooks A110 e documentação de governança.
Ele garante rastreabilidade entre requisitos, owners e evidências de decisão.

## Owners & Watchers
- **Owner primário:** PMO/Governança Técnica.
- **Watchers ativos:** `formal_verification_gate_watch`, `okr_risk_alignment_watch`, `policy_violation_watch`.

## Fluxo operacional
```bash
make lint        # valida inventário, README e presença de hooks
make test        # garante alvos essenciais no Makefile
make build       # gera manifesto com snapshot de governança
make evidence    # sincroniza evidências com ops/evidence/specs.json
```
