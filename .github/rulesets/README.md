# Repository Rulesets & Labels Automation

Este diretório contém o modelo de ruleset utilizado para proteger o branch padrão e branches `release/*`, além do catálogo de labels mantido em `.github/labels.yml`.

## Pré-requisitos

- Git configurado com acesso ao repositório.
- GitHub CLI (`gh`) autenticado com token que possua os escopos `repo` e `administration`.
- `jq` é opcional; os comandos abaixo utilizam apenas `gh`, `git` e `python3`.

## Descobrir o branch padrão

```bash
DEFAULT_BRANCH=$(git remote show origin | awk '/HEAD branch/ {print $NF}')
```

Caso o branch padrão não seja `main`, ajuste o campo `conditions.ref_name.include` do JSON antes de aplicar o ruleset ou utilize o script com `--default-branch`.

## Aplicar labels a partir do catálogo

Os comandos abaixo criam ou atualizam cada label definido em `.github/labels.yml` sem duplicatas:

```bash
while IFS='|' read -r name color description; do
  if gh label view "$name" >/dev/null 2>&1; then
    gh label edit "$name" --color "$color" --description "$description"
  else
    gh label create "$name" --color "$color" --description "$description"
  fi
done <<'LABELS'
$(python3 - <<'PY'
import pathlib, re
path = pathlib.Path('.github/labels.yml')
labels = []
current = {}
for line in path.read_text().splitlines():
    stripped = line.strip()
    if stripped.startswith('- name:'):
        if current:
            labels.append(current)
            current = {}
        current['name'] = stripped.split(':', 1)[1].strip().strip('"')
    elif stripped.startswith('color:'):
        current['color'] = stripped.split(':', 1)[1].strip().strip('"')
    elif stripped.startswith('description:'):
        current['description'] = stripped.split(':', 1)[1].strip()
if current:
    labels.append(current)
for item in labels:
    print(f"{item['name']}|{item['color']}|{item['description']}")
PY)
LABELS
```

## Aplicar o ruleset de proteção

### Usando o endpoint de Rulesets (preferencial)

1. Ajuste o campo `ref_name.include` do arquivo `branch-protection.json`, caso necessário, para incluir o branch padrão detectado.
2. Crie ou atualize o ruleset:

```bash
OWNER_REPO=$(gh repo view --json nameWithOwner --jq .nameWithOwner)
RULESET_NAME=$(python3 - <<'PY'
import json, pathlib
data = json.loads(pathlib.Path('.github/rulesets/branch-protection.json').read_text())
print(data['name'])
PY
)
EXISTING_ID=$(gh api -H "Accept: application/vnd.github+json" \
  repos/$OWNER_REPO/rulesets --jq ".[] | select(.name == \"$RULESET_NAME\") | .id" 2>/dev/null)
PAYLOAD=$(DEFAULT_BRANCH="$DEFAULT_BRANCH" python3 - <<'PY'
import json, pathlib, sys, os
payload = json.loads(pathlib.Path('.github/rulesets/branch-protection.json').read_text())
default_branch = os.environ.get('DEFAULT_BRANCH')
if default_branch:
    refs = payload.setdefault('conditions', {}).setdefault('ref_name', {}).setdefault('include', [])
    default_ref = f"refs/heads/{default_branch}"
    if default_ref not in refs:
        refs.insert(0, default_ref)
json.dump(payload, sys.stdout)
PY)
if [ -n "$EXISTING_ID" ]; then
  printf '%s' "$PAYLOAD" | gh api -X PATCH \
    -H "Accept: application/vnd.github+json" \
    repos/$OWNER_REPO/rulesets/$EXISTING_ID \
    --input -
else
  printf '%s' "$PAYLOAD" | gh api -X POST \
    -H "Accept: application/vnd.github+json" \
    repos/$OWNER_REPO/rulesets \
    --input -
fi
```

### Fallback: proteção de branch clássica

Se o endpoint de rulesets retornar `404` ou `403`, aplique a proteção clássica diretamente no branch padrão:

```bash
CHECKS=$(printf '%s\n' lint-promtool rules-test static-lint schema-validate anti-scans)
JSON=$(CHECKS="$CHECKS" python3 - <<'PY'
import json, os
checks = os.environ['CHECKS'].split('\n')
print(json.dumps({
    "required_status_checks": {
        "strict": True,
        "contexts": [c for c in checks if c]
    },
    "enforce_admins": True,
    "required_pull_request_reviews": {
        "dismiss_stale_reviews": True,
        "require_code_owner_reviews": True,
        "required_approving_review_count": 2
    },
    "allow_force_pushes": False,
    "allow_deletions": False,
    "required_conversation_resolution": True,
    "required_linear_history": True
}))
PY
)
printf '%s' "$JSON" | gh api -X PUT \
  -H "Accept: application/vnd.github+json" \
  repos/$OWNER_REPO/branches/$DEFAULT_BRANCH/protection \
  --input -
# Habilitar assinatura obrigatória (se disponível)
gh api -X POST \
  -H "Accept: application/vnd.github+json" \
  repos/$OWNER_REPO/branches/$DEFAULT_BRANCH/protection/required_signatures || true
```

### Status checks exigidos

Certifique-se de que os nomes dos checks a seguir correspondem exatamente aos jobs definidos na Thread 8:

- `lint-promtool`
- `rules-test`
- `static-lint`
- `schema-validate`
- `anti-scans`

Atualize os nomes se os jobs forem renomeados no futuro.

## Permissões e troubleshooting

- **403 Forbidden**: verifique se o token utilizado possui escopo `administration` e se você é administrador do repositório.
- **422 Unprocessable Entity**: um ou mais status checks não existem. Execute o pipeline para registrar os checks uma vez ou ajuste os nomes.
- **Branch padrão renomeado**: reexecute o comando de detecção e ajuste o ruleset/branch protection antes de aplicar.
- **Ruleset duplicado**: use `gh api repos/$OWNER_REPO/rulesets --jq '.[] | .name'` para listar e remova entradas antigas.
- **Sem permissões suficientes**: o script e os comandos retornam saída informativa e continuam sem aplicar mudanças; utilize `--strict` para forçar erro.

## Script automatizado

O script `scripts/gh_setup_repo_policies.sh` encapsula todo o fluxo com suporte a `--dry-run`, `--strict` e `--default-branch`. Consulte o código para detalhes de saída e códigos de retorno.
