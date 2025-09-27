# CreditEngine$ Working Backwards PRFAQ

## Press Release (2025-04-15)
Today we are announcing the **CreditEngine$ Decision Precision Pack**, a unified release that connects decision pricing,
observabilidade A110 e governança de waivers com rastreabilidade ponta-a-ponta. Clientes internos agora conseguem
aprovar ofertas de crédito com latência p95 abaixo de 0,8 s, com contratos de dados auditáveis e monitoramento automático
contra drift e violações de SLO. O pack inclui notebooks reprodutíveis, documentação operacional e templates de waiver
que blindam riscos de conformidade.

"A cadência de experimentos aumentou 3x porque conseguimos reconstruir o ambiente em minutos e auditar cada decisão com
hashes confiáveis", disse **Joana Ribeiro**, diretora de Crédito. "As squads agora têm autonomia para iterar, sabendo que
os guard-rails de A110 estão amarrados a owners e expirations visíveis".

## Perguntas Frequentes

### What problem are we solving?
As squads enfrentavam dificuldade para manter rastreabilidade Working Backwards: PRFAQ, 6P, notebooks e waivers ficavam
dispersos, sem vínculo com comentários de PR ou relatórios de auditoria. Isso quebrava o princípio de governança descrito
em `agents.md`.

### How do we solve it?
Criamos um pacote de artefatos versionados em `docs/` e `waivers/`, referenciados por `PR_COMMENT.md` e
`ops/reports/repo_audit.json`. O `docs/environment.md` padroniza toolchain e seeds, enquanto os notebooks em
`docs/notebooks/` comprovam latência e conformidade. Os modelos de waiver introduzem owners explícitos, expirations e gates
CI (`watchers.dry`, `hooks.dry`, `evidence.publish`).

### Who is the customer?
- **Primary:** Squads DEC, DATA e ML responsáveis por decisões e monitoramento.
- **Secondary:** SRE/PLAT (observabilidade), SEC/PRIV (waivers e compliance) e governança executiva.

### What does success look like?
- Novos PRs incluem links diretos para PRFAQ, 6P, environment e waivers.
- Auditorias (`ops/reports/repo_audit.json`) mostram owners, seeds e datas de expiração sem lacunas.
- Gates de CI bloqueiam deploys com waivers expirados.

### What is the customer experience?
1. Ao abrir um PR, o time encontra `PR_COMMENT.md` com links de rastreabilidade.
2. Os notebooks (`docs/notebooks/`) permitem reproduzir métricas (latência p95, PSI) e geram figuras anexadas ao PR.
3. Se um requisito precisa de exceção, o time duplica `waivers/template.yaml`, define owner e expiração e registra nos
gates `hooks.dry` e `watchers.dry`.
4. Auditores consultam `ops/reports/repo_audit.json` para verificar conformidade.

### What changed in the environment?
`docs/environment.md` descreve toolchain canônico, seeds globais (`CE_SEED`, `MODEL_SEED`, `AB_SEED`) e os processos de
verificação automática (`runtime_eol_watch`, `model_drift_watch`). Essas informações são sincronizadas com a auditoria e
os comentários de PR para garantir reprodutibilidade.

### What are the top risks and mitigations?
| Risco | Mitigação |
| --- | --- |
| Waivers expirados bloqueando deploy crítico | Jobs CI verificam `waivers/*.yaml` e notificam owners 7 dias antes da expiração. |
| Divergência de seed entre ambientes | Seeds versionadas em `docs/environment.md` e propagadas via `make env.up`. |
| Notebook desatualizado | `PR_COMMENT.md` exige checklist de atualização e `repo_audit.json` aponta última execução. |

### What is out of scope?
- Automação de geração de waivers (manual controlado).
- Alterações nos modelos ONNX; somente documentação e governança foram incluídas.

### How will we measure impact?
- `time-to-preço-válido` p75 monitorado via dashboards referenciados em `repo_audit.json`.
- Número de PRs com `PR_COMMENT.md` preenchido corretamente (>= 95%).
- Auditorias trimestrais sem achados críticos (0 incidentes).

### What are the next steps?
1. Integrar verificação automática de expiração de waivers no pipeline A110.
2. Estender notebooks para simular degradação e failover TWAP.
3. Adicionar templates de runbook específicos para orquestrações de waivers.
