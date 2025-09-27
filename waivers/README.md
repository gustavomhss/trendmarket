# Waivers — Governança CreditEngine$

Este diretório concentra modelos de waivers aprovados pelo comitê A110. Todo waiver deve:
- Conter **owner** (squad + indivíduo responsável).
- Definir **data de expiração** (`expires_at`) e motivação.
- Especificar **gates de CI** afetados (`hooks.dry`, `watchers.dry`, `evidence.publish`).
- Incluir **contramedidas** e plano de retorno.

## Como usar
1. Duplique `template.yaml` ou o modelo específico do domínio.
2. Atualize os campos obrigatórios e assine digitalmente no PR correspondente.
3. Vincule o waiver no `PR_COMMENT.md` e em `ops/reports/repo_audit.json` na seção `waivers`.
4. Abrir ticket de acompanhamento com o owner e data limite.

## Modelos disponíveis
- `template.yaml`: estrutura genérica para novos waivers.
- `data_contract_break.yaml`: exceção temporária para contratos A106/A87/A89.
- `latency_guardrail.yaml`: exceção controlada para `metrics_decision_hook_gap_watch`.

Todos os arquivos são validados por `ops/scripts/gate_a110.sh` durante o CI.
