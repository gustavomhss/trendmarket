# AMM Error Contract Guide

Este guia documenta como evoluir o contrato de erros do AMM de forma segura.  
O contrato é consumido por **UI, API e superfícies de observabilidade** e **deve permanecer estável**.

---

## Como adicionar uma nova variante de erro

1. **Defina a variante no código**
   - Adicione a nova variante ao macro `amm_error_contract!` em [`src/amm/errors.rs`](../../src/amm/errors.rs).
   - Mantenha o enum **sem payload** (somente variantes simples).

2. **Atribua a metadata do contrato**
   - No próprio macro defina `code`, `message` e `http_status` seguindo o padrão:
     - `code` → `CE-AMM-XXXX` (único, nunca reutilizado).
     - `message` → frase curta, neutra, em inglês e terminando com ponto.
     - `http_status` → um dos valores permitidos {400, 403, 404, 409, 500, 502, 503}.
   - As funções `error_code()`, `user_message()` e `http_status()` consomem diretamente esses descritores, garantindo fonte única da verdade.

3. **Atualize o catálogo YAML**
   - Edite [`ops/errors/catalog_amm.yaml`](catalog_amm.yaml) adicionando a nova entrada com `variant`, `code`, `default_message`, `http_status`.
   - Mantenha o bloco `meta` com `{ domain: AMM, prefix: CE-AMM, version: 1 }`.

4. **Regenere o índice JSON para dashboards**
   - Execute [`ops/scripts/generate_amm_error_index.py`](../scripts/generate_amm_error_index.py).
   - O script valida o catálogo e recria [`ops/reports/amm_error_index.json`](../reports/amm_error_index.json) com `meta`, `code`, `message` e `http_status`.

5. **Verifique os testes de contrato**
   - Rode `cargo test --package credit-engine-core amm_error_contract` e `amm_error_catalog`.
   - Os testes percorrem `AmmError::ALL_VARIANTS`, comparam com os descritores e o catálogo YAML, falhando automaticamente se a metadata estiver incompleta.

6. **Documente a evidência**
   - Execute `ops/scripts/package_amm_error_contract.sh` para gerar os bundles (`out/logs`, `out/pkg`, `out/patches`, `out/pr`, `out/jira`).
   - Verifique os arquivos resultantes antes de anexá-los à revisão.

---

## Validação local

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```
