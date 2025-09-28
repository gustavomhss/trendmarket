# Catálogo de erros do AMM

Este diretório materializa o mapeamento oficial entre `AmmError` (Rust) e os códigos operacionais do domínio AMM. O catálogo serve tanto para integrações (tratativa de erros, dashboards, alertas) quanto para governança (auditoria, versionamento e SLOs de resposta).

## Como introduzir um novo `AmmError`

1. **Defina o erro na aplicação**: adicione a nova variante em [`src/amm/errors.rs`](../../src/amm/errors.rs) com a mensagem padrão desejada no `Display`.
2. **Atribua o próximo código**: no arquivo [`catalog_amm.yaml`](catalog_amm.yaml) acrescente uma nova entrada usando o prefixo `CE-AMM` e o próximo número sequencial livre (`CE-AMM-0007`, `CE-AMM-0008`, ...). Preserve `variant`, `code`, `default_message` e `http_status`.
3. **Atualize os testes de cobertura**: inclua a nova variante na lista `expected_catalog()` do teste [`tests/amm_error_catalog.rs`](../../tests/amm_error_catalog.rs) para garantir que o catálogo continue completo.
4. **Regenere o índice JSON** (para dashboards): execute o script [`ops/scripts/generate_amm_error_index.py`](../scripts/generate_amm_error_index.py) a partir da raiz do repositório. Ele consome o YAML e reescreve [`ops/reports/amm_error_index.json`](../reports/amm_error_index.json).
5. **Revise e commite**: garanta que `catalog_amm.yaml`, o teste e o `amm_error_index.json` estejam sincronizados antes de abrir PR.

## Índice JSON para dashboards

O arquivo [`../reports/amm_error_index.json`](../reports/amm_error_index.json) resume `{variant, code, default_message}` e é usado por painéis de observabilidade. Para atualizá-lo:

```bash
./ops/scripts/generate_amm_error_index.py
```

O script não depende de bibliotecas externas e reconstrói o JSON com base no catálogo versionado. Incorpore o resultado no mesmo commit do ajuste no YAML para manter os artefatos alinhados.
