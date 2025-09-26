# A110 — Branch Protection (Required Check)

## Pré-requisitos

Permissão de administrador no repositório e acesso à organização; o workflow deve estar nomeado exatamente como **A110 — Invariants Gate**.

## Via GitHub UI

1. Acesse **Settings → Branches → Branch protection rules** e clique em **Add rule** (ou edite a regra existente para `main`).
2. Defina **Branch name pattern** como `main`.
3. Em **Require status checks to pass before merging**, selecione **A110 — Invariants Gate**.
4. (Opcional) Marque **Require branches to be up to date before merging** para exigir merge com o estado mais recente de `main`.
5. Clique em **Save changes** para aplicar a proteção.

## Via GitHub CLI (gh api)

Execute um PUT em `PUT /repos/:owner/:repo/branches/main/protection`, fornecendo um payload JSON semelhante ao exemplo abaixo:

```json
{
  "required_status_checks": {
    "strict": true,
    "contexts": ["A110 — Invariants Gate"]
  }
}
```

* `strict` é opcional; defina como `true` apenas se quiser exigir que o branch esteja atualizado com `main` antes do merge.
* É necessário possuir escopos `repo` e `admin:repo_hook` no token utilizado pelo `gh`.
* Organizações podem ter políticas que sobreponham configurações locais de proteção.

## Validação

1. Abra um pull request de teste contra `main`.
2. Confirme que o botão **Merge** permanece bloqueado enquanto o workflow **A110 — Invariants Gate** não estiver concluído com sucesso.
3. Para demonstrar o bloqueio, force um teste sintético [P2] a falhar; o PR deve ficar vermelho e impossibilitado de merge até que o check fique verde.

## Reverter a regra

* **UI**: desmarque o status check **A110 — Invariants Gate** na regra de proteção do branch `main` e salve.
* **CLI**: envie um payload para o mesmo endpoint removendo o contexto `"A110 — Invariants Gate"` de `required_status_checks.contexts`.

## Observações

A proteção convive com outras regras (por exemplo, `CODEOWNERS`, aprovações de review e conversas obrigatórias). Priorize o conjunto mínimo de proteções para evitar deadlocks entre exigências conflitantes.