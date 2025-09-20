# Golden set — CPMM (x·y=k) sem taxa


**Grupos**:
- **Simetria**: Rx == Ry, dx variando [1, 1e3, 1e5].
- **Assimetria**: Rx ≫ Ry (50M vs 1k), dx variando [1, 100].
- **Limites**: dx mínimo (1) e dx grande (~50% de Rx), mantendo Ry razoável.
- **Sequência add→swap→remove**: add 10% na mesma razão; checagem de invariância apenas no swap.


**Aceite**: sem taxa, |Δk/k| ≤ 1e-9 em todos os swaps. Implementado via aritmética inteira: |k1-k0| ≤ ⌊k0/1e9⌋.


**Escala**: todos os valores são multiplicados por 1e18 (WAD).
