# CreditEngine ML (Serving & Monitoring)

O diretório `ml/` agrega pipelines de treinamento, export ONNX e monitoramento de drift.
Mantém o guardrail PSI ≤ 0,2 e KS ≤ 0,1 com rollback automático via hooks A110.

## Owners & Watchers
- **Owner primário:** Squad ML (Model Lifecycle).
- **Watchers ativos:** `model_drift_watch`, `ab_srm_watch`, `runtime_eol_watch`, `dep_vuln_watch`.

## Fluxo operacional
```bash
make lint        # valida README, inventário e lockfile
make test        # checa targets essenciais e cobertura de watchers
make build       # gera manifest de build para servir modelos
make evidence    # publica evidências no inventário central
```
