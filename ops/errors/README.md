# AMM Error Contract Guide

Este guia documenta como evoluir o contrato de erros do AMM de forma segura.  
O contrato é consumido por **UI, API e superfícies de observabilidade** e **deve permanecer estável**.

---

## Como adicionar uma nova variante de erro

1. **Defina a variante no código**  
   - Adicione a nova variante no enum `AmmError` em [`src/amm/errors.rs`](../../src/amm/errors.rs).  
   - Mantenha o enum **sem payload** (somente variantes simples).  

2. **Atribua a metadata do contrato**
   - O macro `amm_error_contract!` deve receber a nova variante e todos os campos.
   - Forneça mapeamentos em:
     - `error_code()` → siga o padrão `CE-AMM-XXXX` (único, nunca reutilizado).
     - `user_message()` → frases curtas, neutras, em inglês, terminando com ponto.
     - `http_status()` → código HTTP correspondente (400/403/404/409/500/502/503).
     - `variant_name()` → nome legível da variante.

3. **Atualize o catálogo YAML**  
   - Edite [`ops/errors/catalog_amm.yaml`](catalog_amm.yaml) adicionando a nova entrada com os campos:  
     - `variant`, `code`, `default_message`, `http_status`.  

4. **Regenere o índice JSON para dashboards**  
   - Execute o script [`ops/scripts/generate_amm_error_index.py`](../scripts/generate_amm_error_index.py).  
   - Ele consome o catálogo YAML e reescreve [`ops/reports/amm_error_index.json`](../reports/amm_error_index.json), usado em painéis de observabilidade.  

5. **Atualize os testes**
   - Ajuste [`tests/amm_error_contract.rs`](../../tests/amm_error_contract.rs) para que o conjunto retornado por `AmmError::descriptors()` e o catálogo YAML permaneçam sincronizados.

6. **Documente a evidência**  
   - Rode `ops/scripts/package_amm_error_contract.sh` para regenerar os bundles (`logs`, `patches`, `artifacts`).  
   - Isso garante que formatador, linter e testes capturem a nova variante.  

---

## Validação local

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
