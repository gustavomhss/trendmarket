# ADR-0002 — Taxa e fórmula de swap (CPMM)

**Status**: Proposto → (aprovar nesta subtarefa)  
**Escopo**: AMM CPMM (`x·y = k`) — cálculo de `amountOut` e `amountIn` com **taxa sobre o input**.  
**Depende**: ADR-0001 (escala 1e18, U256, arredondamento).

## 1) Decisões (resumo)
- **Taxa sobre o input** (`fee_ppm`, 0..1e6):  
  `dx_fee = ceil(dx * fee_ppm / 1e6)` → `dx_net = dx − dx_fee`.
- **Arredondamento (mesma política do ADR-0001)**:  
  fronteira: `amount_out = floor`, `amount_in = ceil`, `fee = ceil`;  
  intermediários: **nearest (ties-to-even)**.
- **Invariante**:  
  sem taxa → alvo `|Δk/k| ≤ 1e−9`;  
  com taxa → **`k' ≥ k`** (parte da taxa fica na pool).

## 2) Fórmulas — X → Y (amountOut)
Dado `(x, y)`, `k=x*y`, `dx` bruto:
1. `dx_fee = ceil(dx * fee_ppm / 1e6)`
2. `dx_net = dx − dx_fee`  (exigir `dx_net ≥ 1`)
3. `x' = x + dx_net`
4. `y* = k / x'`  (divisão 256 bits, **nearest-even**)
5. `amount_out = floor(y − y*)`
6. `y' = y − amount_out` (exigir `y' ≥ min_reserve`)

## 3) Resolver amountIn para obter um `dy` (mínimo dx)
1. Sem taxa: `dx_net = ceil( x * dy / (y − dy) )`
2. Converter para bruto: `dx = ceil( dx_net * 1e6 / (1e6 − fee_ppm) )`
3. Verifique: `dx − ceil(dx*fee_ppm/1e6) ≥ dx_net`; se não, **dx++** e repita.
4. Aplique as fórmulas de cima e confirme `amount_out ≥ dy`.

## 4) Casos-limite (regras)
- `dx = 0` → rejeitar.
- `dx_net = 0` (taxa comeu tudo) → rejeitar com erro claro.
- `dy ≥ y − min_reserve` → rejeitar (não esvaziar pool).
- Sempre usar `U256` nos produtos/divisões; _downcast_ checado.
- `fee_ppm = 1_000_000` (100%) → rejeitar.

## 5) Exemplos worked‑out (para virar golden depois)
> Notação legível (sem 1e18). `fee=3000 ppm` = 0,30% quando indicado.

**E1 — Simétrico, sem taxa (amountOut)**  
`x=y=1_000_000`, `dx=10_000`, `fee=0`  
`dx_net=10_000`, `x'=1_010_000`, `y*≈990_099.0099`,  
`out=floor(1_000_000−990_099.0099)=9_900`, `y'=990_100`.  
`|Δk/k| ≈ 1e−6` (floor na fronteira).

**E2 — Simétrico, com taxa 0,30% (amountOut)**  
`x=y=1_000_000`, `dx=10_000`, `fee=3000`  
`dx_fee=30` → `dx_net=9_970`, `x'=1_009_970`, `y*≈990_129.317…`,  
`out=floor(1_000_000−990_129.317…)=9_870`; **`k' ≥ k`**.

**E3 — amountIn para `dy=9_870` (com taxa)**  
`x=y=1_000_000`, `fee=3000`, `dy=9_870`  
`dx_net = ceil(1_000_000*9_870/(1_000_000−9_870)) = 9_970`  
`dx = ceil(9_970*1e6/(1e6−3000)) = 10_000`  
Cheque: `dx_fee=30` → `dx_net=9_970` → aplica (2) → `out=9_870`.

**E4 — Assimétrico, sem taxa**  
`x=1_000`, `y=1_000_000_000`, `dx=100`, `fee=0`  
`x'=1_100`, `y*≈909_090_909.09…`, `out≈90_909_090`.

**E5 — Entrada mínima com taxa (rejeitar)**  
`x=5_000_000`, `y=4_000_000`, `dx=1`, `fee=3000`  
`dx_fee=1` → `dx_net=0` ⇒ **erro: input efetivo nulo**.

## 6) Done (critérios desta subtarefa)
- ADR com fórmulas finais e onde a taxa incide.
- ≥3 exemplos (com/sem taxa) incluídos.
- Casos‑limite listados.

