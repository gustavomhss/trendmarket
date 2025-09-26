# Playbook de evolução do catálogo de erros do AMM

## Quando adicionar ou reutilizar códigos
- **Adicionar novo código** sempre que a causa raiz ou remediação forem diferentes das existentes.
- **Reutilizar código** apenas quando o novo caso for semanticamente idêntico ao já documentado (mesmo título/mensagem/contexto esperado).
- Evite alterar significado de códigos já emitidos; preserve retrocompatibilidade.

## Checklist para PRs
1. Atualizar `src/amm/error_catalog.rs` com nova variante e garantir que `AmmErrorCode::all()` está em ordem estável.
2. Ajustar `src/amm/error.rs` e `src/amm/error_map.rs` conforme necessário (mensagens, contexto, mapeamentos).
3. Incluir ou atualizar testes:
   - `tests/amm_error_catalog.rs`
   - `tests/amm_error_roundtrip.rs`
   - `tests/amm_error_ui_strings.rs`
4. Revisar docs `docs/amm-error-catalog.md` e este playbook.
5. Validar schema `schemas/amm_error.schema.json` (novos exemplos, campos obrigatórios).
6. Garantir que o gate A110 permanece verde e que os novos códigos estão cobertos em evidências.

## Comunicação
- Registrar mudança no CHANGELOG do produto.
- Informar time de UI/UX para planejar textos traduzidos e comportamentos específicos.
- Avisar Observabilidade/Telemetria para ajustar alertas e dashboards que filtram por código.

## Depreciação suave
- Mantenha códigos antigos disponíveis para interpretação histórica.
- Documente códigos substituídos e migrações recomendadas nas notas de versão.
- Somente remova um código após garantir que nenhuma integração ativa o consome.