# Rollback A110 — Playbook

## Quando acionar

- Regressão P1/P2 confirmada pelo A110 imediatamente após merge ou deploy.
- Queda do SLO primário (latência, erro, throughput) atribuída ao commit recém-mergeado.
- Indicadores de risco do gate (anomalias de invariantes críticas) sinalizando falha com impacto alto.
- Relatos de clientes internos ou externos que reproduzem a regressão no build atual.
- Confirmada indisponibilidade de fluxos essenciais protegidos pelo A110.

## Decidir: Rollback x Forward-fix

| Fator | Questão | Sinal para rollback |
| --- | --- | --- |
| Tempo de correção | Fix pode ser entregue em até 30 minutos? | Não |
| Blast radius | Impacto atual afeta mais de um domínio crítico ou clientes externos? | Sim |
| Janela operacional | Existe janela de risco (fim de expediente, alto volume, freeze)? | Sim |
| Dependências | Rollback é autônomo (sem coordenação complexa)? | Sim |
| Observabilidade | Dados inconclusivos para validar fix forward rapidamente? | Sim |

Checklist:

- [ ] Se a resposta acima preferir rollback (fix > 30 min ou impacto alto), iniciar playbook imediatamente.
- [ ] Confirmar com o Incident Commander / on-call antes de executar.
- [ ] Notificar stakeholders críticos sobre a decisão.

## Procedimentos

### 1) GitHub UI (Revert do PR)

1. Abrir o PR mergeado que introduziu a regressão (A110 indica o número no alerta).
2. Localizar o merge commit na UI (seção "Commits" do PR) e clicar em "Revert".
3. Confirmar a criação automática de um novo PR com o commit de rollback.
4. Editar título e descrição para indicar rollback emergencial (ex.: "Rollback: <PR original>").
5. Solicitar revisão mínima (1 reviewer) e mergear com prioridade máxima.
6. Acompanhar o deploy automático resultante do merge do PR de rollback.

### 2) Git CLI

1. Identificar o merge:
   - `git log --merges --oneline` para encontrar o commit.
   - Ou `gh pr view -w <numero_pr>` para abrir detalhes no navegador.
2. Executar rollback:
   - `git checkout main`
   - `git pull`
   - `git revert -m 1 <merge_sha>`
   - Resolver conflitos, se existirem.
   - `git push origin main`
3. Para squash merge (commit único), executar `git revert <commit_sha>` sem `-m 1`.
4. Se preferir PR, criar branch temporária, abrir PR de rollback e seguir política padrão de revisão rápida.

### 3) Release/Deploy

1. Conferir tags disponíveis: `git tag --sort=-creatordate | head`.
2. Validar release anterior no GitHub: `gh release view <tag-anterior>`.
3. Reaplicar artefatos, se necessário:
   - `gh release download <tag-anterior> --dir ./releases/<tag-anterior>`
   - Verificar integridade dos binários/containers.
4. Criar release de rollback (quando requerido pelo fluxo):
   - `gh release create <tag-anterior>-revert --notes "Rollback da release <tag-anterior> devido a regressão detectada pelo A110." --target <commit_revertido>`
5. Reimplantar a versão anterior conforme o pipeline (ex.: reexecutar workflow de deploy apontando para `<tag-anterior>`).
6. Confirmar que todos os ambientes afetados receberam a versão revertida.

## Validação pós-rollback

- A110 verde no commit revertido (sem regressões P1/P2).
- Smoke tests automatizados executados com sucesso (pipeline CI ou scripts manuais).
- Dashboards de monitoramento (Prometheus/Grafana) mostram métricas normalizadas.
- Traços Jaeger voltam a chegar com latência esperada.
- Endpoints de saúde (`/health`, `/ready`) respondem OK.
- Verificar logs para ausência de novos erros críticos.

## Comunicação

- Comentário no PR de rollback: "Rollback executado às <hora> UTC. Regressão confirmada no A110 (<link alerta>). SLO normalizado."
- Atualização no CHANGELOG: seção "Fixes" com referência ao rollback e ao PR original.
- Anúncio interno (Slack/Email):
  - Impacto observado (serviços/usuários afetados).
  - Causa preliminar (commit/PR que introduziu regressão).
  - Mitigação adotada (rollback, versão reinstalada).
  - Próximos passos (investigação root cause, follow-up tickets).
- Registrar incidente no sistema de tracking (ex.: PagerDuty/Jira) com status mitigado.

## Prevenção

- Adicionar testes automatizados (golden/property-based) cobrindo o cenário falho.
- Avaliar ativação de feature flag para limitar rollout futuro.
- Melhorar alertas do A110 (limiares, notificações redundantes).
- Revisar hardening do gate (validar invariantes pré-merge, canary automated).
- Garantir cobertura de seeds e dados críticos nos ambientes de teste.
- Abrir tickets de follow-up e atribuir responsáveis com prazo.

## Apêndice

- Encontrar merge commit recente: `git log --merges --since="24 hours ago" --oneline`.
- Listar releases disponíveis: `gh release list --limit 20`.
- Verificar deploy ativo em Kubernetes: `kubectl rollout status deployment/<servico>`.
- Conferir histórico de alertas do A110: acessar dashboard `<url_interna>`.
- Checklist rápido de comandos úteis:
  - `git revert --continue`
  - `gh pr create --title "Rollback" --body "Rollback automatizado"`
  - `kubectl rollout undo deployment/<servico>`