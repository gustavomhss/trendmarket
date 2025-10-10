# OBS-3 Local Verifier

## Visão geral
O verificador local `scripts/obs3_all_checks.sh` agrega todas as verificações obrigatórias do pack OBS-3 em uma única execução. Os alvos `make lint`, `make evidence` e `make pr-check` fornecem experiências diferentes:

- `make lint`: focado em validações estáticas (Prometheus, YAML, shell e Python) para feedback rápido durante o desenvolvimento.
- `make evidence`: executa o runner de scrapes em modo DEV e, se disponíveis, encadeia as etapas de qualidade, hashing e verificação de manifesto para gerar artefatos.
- `make pr-check`: invoca o verificador único, cobrindo dependências, anti-scans, linters, validações de Prometheus e verificações de esquema.

## Pré-requisitos
Instale e mantenha atualizados os binários abaixo antes de executar os alvos:

- `promtool` (Prometheus Toolkit)
- `yamllint`
- `shellcheck`
- `ruff`
- `python3`

Consulte os gerenciadores de pacotes locais ou a documentação oficial de cada ferramenta para instruções de instalação.

## Como usar
Com o repositório configurado, execute os alvos conforme a necessidade:

```sh
make lint
make evidence
make pr-check
```

Os comandos aceitam variáveis sobrepostas, por exemplo `PROMTOOL=/path/para/promtool make pr-check`.

## Saídas e artefatos
- Logs e PIDs residem em `out/obs_gatecheck/logs/`.
- Evidências e manifestos ficam em `out/obs_gatecheck/evidence/`.
- O verificador imprime um resumo RAG por etapa e um consolidado ao final.

## Política de falhas
- **FAIL**: interrompe o fluxo (código de saída `1`, exceto dependências ausentes, que retornam `7`). É obrigatório corrigir antes de prosseguir.
- **WARN**: sinaliza itens não bloqueantes (por exemplo, ausência de manifestos gerados). O processo finaliza com código `0`, mas os avisos devem ser acompanhados.

## Boas práticas
- Execute `make lint` e `make pr-check` antes de cada commit ou pull request.
- Gere evidências com `make evidence` sempre que atualizar scrapes, regras ou manifestos.
- Atualize as ferramentas de linting e Prometheus periodicamente para manter paridade com a pipeline oficial.
- Revise os artefatos em `out/obs_gatecheck/` e versionamentos de hash antes de anexar evidências em PRs.
