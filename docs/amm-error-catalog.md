# Catálogo de erros do AMM

| Código | Título | Mensagem PT | Placeholder(s) |
| --- | --- | --- | --- |
| AMM-0001 | Quantidade zerada | amount deve ser > 0 | — |
| AMM-0002 | Reserva zerada | reserve deve ser > 0 | — |
| AMM-0003 | Reserva mínima violada | reserva ficaria abaixo do mínimo | — |
| AMM-0004 | Overflow numérico | overflow/underflow numérico | — |
| AMM-0005 | Input efetivo zerado | input efetivo após taxa é 0 | — |

## Como usar
- Prefira as macros `amm_err!` e `amm_bail!` para construir erros com contexto em cadeia.
- Adicione pares `chave => valor` diretamente ou via bloco `{ "chave" => valor }`.
- Mensagens para UI devem permanecer curtas; detalhes complementares vão no `context`.

## Serialização
- `to_user_string()` produz `[AMM-XXXX] mensagem` sem quebras de linha ou tabs.
- `to_log_json()` segue `schemas/amm_error.schema.json` com chaves fixas `code`, `title`, `message`, `context`.
- Placeholders não resolvidos permanecem literais; valores de contexto são sanitizados e truncados com estabilidade determinística.

## Boas práticas
- Nunca reutilize códigos existentes para significados diferentes; adicione novas variantes.
- Não inclua PII ou segredos em mensagens nem no `context`.
- Sempre adicione testes novos (D/E/F) ao introduzir variantes ou alterar mensagens.
- Atualize este documento, o playbook e o schema JSON antes de abrir PR.