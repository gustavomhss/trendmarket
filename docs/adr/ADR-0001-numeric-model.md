# ADR-0001 — Modelo numérico & política de arredondamento (CPMM)

**Status**: Proposto → (aprovar nesta subtarefa)  
**Escopo**: AMM CPMM (`x·y=k`) — swap, liquidez, preço/slippage.

## Contexto
Precisamos de um modelo **determinístico** e **reprodutível**, sem floats/NaN, com **uma política única de arredondamento** aplicada em todo o core para evitar drift numérico e inconsistência entre funções.

## Decisão
1) **Escala fixa (WAD)**: inteiros sem sinal com escala **S = 10^18** (1 unidade = 1e18 “wei”). Tipo lógico: `u128`.
2) **Intermediários 256 bits**: multiplicações/divisões em `U256`; _downcast_ checado para `u128`.
3) **Política de arredondamento (única)**  
   | Operação | Política |
   |---|---|
   | `amount_out` (swap X→Y) | **floor** |
   | `amount_in`  (alvo DY)  | **ceil**  |
   | `fee(input)`            | **ceil**  |
   | `shares_mint`           | **floor** |
   | `shares_burn → amounts_out` | **floor** |
   | preços/slippage internos | **nearest (ties-to-even)** |
4) **Taxa**: cobrada **sobre o input**. `dx_fee = ceil(dx * fee_ppm / 10^6)`; `dx_net = dx - dx_fee`.
5) **Limites**: `min_reserve = 1 * S`; rejeitar operações que resultem em `dx_net = 0` ou reservas < `min_reserve`.

## Especificação (resumo)
**Swap X→Y**  
1. `dx_fee = ceil(dx * fee_ppm / 10^6)`  
2. `dx_net = dx - dx_fee`  
3. `x' = x + dx_net`  
4. `y* = (x*y) / x'` (divisão em 256 bits com nearest-even)  
5. `amount_out = floor(y - y*)`  
6. `y' = y - amount_out` (validar `y' ≥ min_reserve`)

**Liquidez**  
- **Mint inicial**: `shares = floor( sqrt(x*y) )` (raiz inteira ↓ em 256 bits)  
- **Add**: `shares = floor( min( dx * totalShares / x,  dy * totalShares / y ) )`  
- **Remove**: `amount_x = floor( x * shares / totalShares )` e idem para `y`.

## Precisão (|Δk/k|)
- Sem taxa: alvo **`|Δk/k| ≤ 1e−9`** por operação (limite de engenharia); com S=1e18 tende a ser ≪ 1e−9.
- Com taxa: **`k' ≥ k`** por construção (parte da taxa fica na pool).

## Guardrails de erro
- Erros: `ErrZeroReserve`, `ErrOverflow`, `ErrInsufficientLiquidity`, `ErrMinReserveBreached`.  
- Usar `checked_*` e `U256` em operações críticas; validar antes do _downcast_.

## Exemplos worked-out (didáticos; sem escala S para leitura)
**Caso 1 — Simétrico, fee=0**  
`x=y=1_000_000`, `dx=10_000` → `k=10^12`, `x'=1_010_000`, `y*≈990_099.0099`, `amount_out=floor(9_900)=9_900`, `y'=990_100`. `Δk/k ~ +1e−6` (floor na fronteira).

**Caso 2 — Simétrico, fee=0.30% (3000 ppm)**  
`dx_fee=30` → `dx_net=9_970` → `x'=1_009_970` → `amount_out=9_870`; **`k' ≥ k`**.

**Caso 3 — Assimétrico, fee=0**  
`x=1_000`, `y=1_000_000_000`, `dx=100` → `amount_out≈90_909_090` (nearest-even nos intermediários; floor na saída).

**Caso 4 — Entrada mínima, com taxa**  
`dx=1` → `dx_fee=1` → `dx_net=0` ⇒ **rejeitar** (input efetivo nulo).

**Caso 5 — Sequência add→swap→remove**  
Pool inicial `(1_000_000, 1_000_000)`; swap `dx=10_000, fee=0.30%` → remove 10% shares. Mantém políticas de floor/ceil; **`k'` não diminui** com taxa.

## Consequências
Determinismo, viés controlado, portável entre linguagens (inteiros + escala). Custo: uso de `U256` para segurança de overflow.

## Próximos
CRD-7-02 — fórmula detalhada de swap (derivação e provas).
