# Triple Review — Thread B (Política de Rounding)

## Lógica
- [x] Conferi cada estágio no código-fonte (`src/amm/swap.rs`, `src/amm/pricing.rs`, `src/amm/liquidity.rs`) contra `out/docs/rounding_matrix.csv` para garantir que direção e momento coincidam.
- [x] Validei que todos os `operation_id` listados no CSV aparecem na tabela da seção `Política de Rounding` do README.
- [x] Revisei os casos de borda (overflow, zero, min reserve) comparando com os retornos `AmmError` no código.

## Sintaxe
- [x] CSV e JSON validados com `python3` (load + dump) sem erros.
- [x] Markdown renderiza corretamente (tabela com 9 colunas, listas e inline code revisados manualmente).
- [x] Comentários `<!-- SECTION:ROUNDING -->` mantidos intactos.

## Identação e Estilo
- [x] Tabela usa alinhamento padrão pipe do projeto e largura < 120 colunas.
- [x] Terminologia (`ceil`, `floor`, `nearest-even`, `MIN_RESERVE`) consistente entre README e artefatos.
- [x] Arquivos novos com codificação UTF-8 e fim de linha LF.
