# CreditEngine Ops Tests & Probes

O diretório `ops/tests/` hospeda probes sintéticos, geradores de Q/A e suites de conformidade para os hooks A110.
Ele garante que cobertura de watchers e gatilhos permaneça ≥ 100% com owners explícitos.

## Owners & Watchers
- **Owner primário:** Squad SRE/QA Operacional.
- **Watchers ativos:** `metrics_decision_hook_gap_watch`, `slo_budget_breach_watch`, `formal_verification_gate_watch`.

## Fluxo operacional
```bash
make lint        # valida inventário, README e lockfile
make test        # confirma targets essenciais no Makefile
make build       # gera manifest com probes e cobertura
make evidence    # publica evidências em ops/evidence/ops-tests.json
```
