# OpenTelemetry Collector Contrib Distro

This distribution contains all the components from both the [OpenTelemetry Collector](https://github.com/open-telemetry/opentelemetry-collector) repository and the [OpenTelemetry Collector Contrib](https://github.com/open-telemetry/opentelemetry-collector-contrib) repository. This distribution includes open source and vendor supported components.

## Recommendation

As this distribution contains many components, it is a good starting point to try various configurations. However, when running in production, it is recommended to limit the collector to contain only the components necessary for an environment. Some reasons to do this:

* reduce the size of the collector, reducing deployment times for the collector
* improve the security of the collector by reducing the available attack surface area

Building a [custom collector](https://opentelemetry.io/docs/collector/custom-collector/) can be achieved using the [OpenTelemetry Collector Builder](https://github.com/open-telemetry/opentelemetry-collector/tree/main/cmd/builder).

## Components

The full list of components is available in the [manifest](manifest.yaml)

## ADRs & CI

<!-- SECTION:ADRCI -->

### ADRs relevantes

Tabela com ADRs que impactam este módulo.

|  ID  | Título                                                         | Data       |  Status  | Área | Link                                                                                                                     |
| :--: | :------------------------------------------------------------- | :--------- | :------: | :--- | :----------------------------------------------------------------------------------------------------------------------- |
| 0001 | ADR-0001 — Modelo numérico & política de arredondamento (CPMM) | 2025-09-19 | Proposed | amm  | [ADR-0001-numeric-model.md](docs/adr/ADR-0001-numeric-model.md#adr-0001-modelo-numerico-politica-de-arredondamento-cpmm) |
| 0002 | ADR-0002 — Taxa e fórmula de swap (CPMM)                       | 2025-09-19 | Proposed | amm  | [ADR-0002-swap-fee-and-formula.md](docs/adr/ADR-0002-swap-fee-and-formula.md#adr-0002-taxa-e-formula-de-swap-cpmm)       |

> Para o índice completo, veja `docs/adr/INDEX.md` (se existir) ou a pasta `docs/adr/`.

### Integração Contínua (CI)

Badges e links para workflows/pipelines.

[![CI](https://github.com/gustavomhss/trendmarket/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gustavomhss/trendmarket/actions/workflows/ci.yml)
[![Docs Guard (Agents)](https://github.com/gustavomhss/trendmarket/actions/workflows/docs-guard-agents.yml/badge.svg?branch=main)](https://github.com/gustavomhss/trendmarket/actions/workflows/docs-guard-agents.yml)

**Workflows/pipelines**

| Nome                | Arquivo                                 | Link                                                                                                                                                                     |
| :------------------ | :-------------------------------------- | :----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CI                  | .github/workflows/ci.yml                | [https://github.com/gustavomhss/trendmarket/actions/workflows/ci.yml](https://github.com/gustavomhss/trendmarket/actions/workflows/ci.yml)                               |
| Docs Guard (Agents) | .github/workflows/docs-guard-agents.yml | [https://github.com/gustavomhss/trendmarket/actions/workflows/docs-guard-agents.yml](https://github.com/gustavomhss/trendmarket/actions/workflows/docs-guard-agents.yml) |

**Como verificar o status**

* Abra o workflow **CI** e valide os jobs `gate-a110` e `sbom` na branch `main` ou na branch desta thread.
* Consulte os artefatos anexados (`gate_a110.log`, `sbom.json`) para cruzar com os comandos descritos em *Build, Test & Bench*.
* No workflow **Docs Guard (Agents)**, confirme que o rótulo `docs::guard::agents` está aplicado antes do merge.

**Observações**

* O workflow **CI** roda continuamente em pushes e pull requests para garantir Gate A110 e gerar o snapshot de SBOM.
* O workflow **Docs Guard (Agents)** bloqueia PRs sem o rótulo obrigatório quando tocam documentação regida por `agents.md`.
* Para replicar localmente, execute os comandos da seção *Build, Test & Bench* antes de abrir o PR.

<!-- END:ADRCI -->

### Rules for Component Inclusion

* Include all extensions at [Alpha stability](https://github.com/open-telemetry/opentelemetry-collector#alpha) or higher and pipeline components that have at least 1 signal at [Alpha stability](https://github.com/open-telemetry/opentelemetry-collector#alpha) or higher.

## Operational Governance: Watchers & Gate A110

This repository now ships with first-class governance artifacts to guarantee that the mandatory watchers and A110 hooks defined in `agents.md` stay green across environments.

### Inventory

* **Watchers:** Domain-specific configurations live under [`ops/watchers/`](ops/watchers). Each `.yml` file describes the KPI, window, action, owner, and rollback policy for the mandatory watches covering DEC, PM, ML, DATA, PLAT, FE, SEC/PRIV and INT domains. These inventories are the single source of truth consumed by `ops/scripts/watchers_dry_run.py`, which now enumerates every `.yml` file in that directory when generating `ops/reports/watchers_dry.json`.
* **Gate A110 hooks:** The consolidated mapping is defined in [`ops/hooks/a110.yml`](ops/hooks/a110.yml). Every watcher is wired to the correct A110 hook, including the required thresholds and evidence links.

### Local enablement

1. Ensure Python 3.11+ is available (the same interpreter used for the existing automation).
2. Run the dry-run validators before coding:

   ```bash
   make watchers.dry
   make hooks.dry
   ```

   Both commands surface missing watchers, invalid parameters, or mismatched hook bindings with actionable error messages.
3. For a full Gate A110 rehearsal (watchers + hooks + Rust invariants), execute:

   ```bash
   make gate.a110
   ```

   The script aborts immediately if any watcher or hook validation fails, ensuring issues are detected before the Rust suites start.

### CI guidance

* The canonical A110 pipeline (`scripts/a110_run_invariants.sh`) now invokes `watchers.dry` and `hooks.dry` automatically. Any gap in coverage causes the gate to fail with a non-zero exit code, guaranteeing enforcement during pull requests.
* Integrate the Make targets above (or call the scripts directly) in bespoke CI systems to keep behaviour consistent between local, nightly, and production promotion flows.
* When adding a new watcher or hook, commit the YAML changes together with updated documentation so that the dry-run validators remain the single source of truth.

## Política de Rounding

<!-- SECTION:ROUNDING -->

Os cálculos do módulo de AMM operam em escala fixa (`WAD = 1e18`, `PPM = 1e6`), portanto cada etapa precisa tratar frações de forma determinística para evitar over-credit aos LPs ou under-collect de taxas. A política adota **ceil** sempre que o protocolo precisa arrecadar algo (ex.: fee ou tolerância de slippage) e **floor** quando devolve valores ao usuário, enquanto divisões de preço usam **nearest-even** para permanecerem neutras. Essa combinação garante que a soma das reservas nunca aumente por artefato de arredondamento e que a experiência da UI permaneça previsível.

Como guardrail adicional, todas as divisões críticas usam `U256` antes do downcast para `u128`, evitando overflow silencioso, e erro explícito (`AmmError`) substitui qualquer saturação não controlada. Quando uma operação admite um empate, aplicamos `half-even` para evitar viés sistemático entre as direções de negociação. Os detalhes completos ficam rastreados em `out/docs/rounding_matrix.csv` e `out/docs/rounding_rules.json`.

| Operação (ID)                    | Estágio                       | Base    | Decimais | Direção | Momento                  | Regra de Empate | Racional (econômico/UX)                                                             | Referência                    |
| -------------------------------- | ----------------------------- | ------- | -------- | ------- | ------------------------ | --------------- | ----------------------------------------------------------------------------------- | ----------------------------- |
| `swap_get_amount_out`            | apply_input_fee               | Q128.18 | 18       | ceil    | before_net_input         | n/a             | Garante arrecadação mínima da taxa e evita subcobrança percebida pelo trader        | `src/amm/swap.rs:27..35`      |
| `swap_get_amount_out`            | solve_invariant               | Q128.18 | 18       | bankers | after_net_input          | half-even       | Mantém o invariável `k` simétrico e entrega cotação consistente                     | `src/amm/swap.rs:57..59`      |
| `swap_get_amount_out`            | finalize_out_amount           | Q128.18 | 18       | floor   | before_min_reserve_check | n/a             | Impede entregar mais Y do que o pool suporta após validar `MIN_RESERVE`             | `src/amm/swap.rs:61..68`      |
| `swap_get_amount_in`             | compute_net_target            | Q128.18 | 18       | ceil    | before_fee_gross_up      | n/a             | Garante `dx_net` suficiente para comprar `dy` sem reprocessar buscas                | `src/amm/swap.rs:94..96`      |
| `swap_get_amount_in`             | gross_up_fee                  | Q128.18 | 18       | ceil    | before_search            | n/a             | Faz o gross-up da taxa e força `hi ≥ 1`, evitando loops infinitos                   | `src/amm/swap.rs:105..111`    |
| `pricing_min_out_with_tolerance` | apply_slippage_discount       | Q128.18 | 18       | floor   | after_amount_out         | n/a             | Aplica desconto conservador antes do check de slippage para proteger LPs e usuários | `src/amm/pricing.rs:84..88`   |
| `pricing_max_in_with_tolerance`  | apply_slippage_markup         | Q128.18 | 18       | ceil    | after_amount_in          | n/a             | Gross-up garante margem suficiente para honrar a tolerância informada               | `src/amm/pricing.rs:107..110` |
| `pricing_execution_price_x_to_y` | compute_execution_price       | Q128.18 | 18       | bankers | after_amount_out         | half-even       | Preço percebido sem viés direcional mesmo em empates                                | `src/amm/pricing.rs:38..40`   |
| `pricing_spot_price_x_in_y`      | compute_spot_price            | Q128.18 | 18       | bankers | after_reserve_validation | half-even       | Mantém simetria entre pares e evita drift no spot                                   | `src/amm/pricing.rs:19..22`   |
| `pricing_spot_price_y_in_x`      | compute_spot_price            | Q128.18 | 18       | bankers | after_reserve_validation | half-even       | Idem acima, invertendo o par                                                        | `src/amm/pricing.rs:26..29`   |
| `pricing_slippage_ppm_x_to_y`    | normalize_slippage_ratio      | Q32.6   | 6        | bankers | after_execution_price    | half-even       | Normaliza `spot` vs `exec` sem viés e sem inflar alertas                            | `src/amm/pricing.rs:52..60`   |
| `pricing_slippage_ppm_x_to_y`    | clamp_slippage_bounds         | Q32.6   | 6        | none    | after_ratio              | n/a             | Clampa em `0..PPM_SCALE` para evitar overflow e mensagens inválidas                 | `src/amm/pricing.rs:61..64`   |
| `liquidity_initial_mint`         | sqrt_liquidity_floor          | Q128.18 | 18       | floor   | after_invariant_product  | n/a             | Shares iniciais nunca excedem `√(x·y)` real, protegendo o pool                      | `src/amm/liquidity.rs:35..39` |
| `liquidity_add_liquidity`        | proportional_allocation_floor | Q128.18 | 18       | floor   | after_ratio_projection   | n/a             | Mint limitado pelo braço mais curto para evitar diluição injusta                    | `src/amm/liquidity.rs:50..55` |
| `liquidity_remove_liquidity`     | redeem_allocation_floor       | Q128.18 | 18       | floor   | before_min_reserve_check | n/a             | Saques respeitam `MIN_RESERVE`, evitando underflow de reservas                      | `src/amm/liquidity.rs:72..83` |

**Ordem do pipeline por operação**

* `swap_get_amount_out`: validar reservas → `apply_input_fee` (ceil) → atualizar `x'` → `solve_invariant` (nearest-even) →
  `finalize_out_amount` (floor) → checar `MIN_RESERVE`.
* `swap_get_amount_in`: validar entradas (`dy`, reservas, fee) → `compute_net_target` (ceil) → `gross_up_fee` (ceil, força `hi ≥ 1`) →
  expandir limite superior → busca binária consumindo `get_amount_out` até `out ≥ dy`.
* `pricing_min_out_with_tolerance`: reusar `get_amount_out` → `apply_slippage_discount` (floor) → devolver `min_out` para UI.
* `pricing_max_in_with_tolerance`: reusar `get_amount_in` → `apply_slippage_markup` (ceil) → devolver `max_in` para UI.
* `pricing_execution_price_x_to_y`: validar entradas → chamar `get_amount_out` → `compute_execution_price` (nearest-even) → exibir WAD.
* `pricing_spot_price_x_in_y` / `pricing_spot_price_y_in_x`: validar reservas → `compute_spot_price` (nearest-even) → publicar WAD.
* `pricing_slippage_ppm_x_to_y`: obter `spot` e `exec` → `normalize_slippage_ratio` (nearest-even) → `clamp_slippage_bounds` (`0..1e6`).
* `liquidity_initial_mint`: validar reservas → calcular `x·y` em `U256` → `sqrt_liquidity_floor` → checar `shares > 0`.
* `liquidity_add_liquidity`: validar entradas → projetar contribuição em cada braço (floor) → `proportional_allocation_floor` (min) → validar
  crescimento das reservas.
* `liquidity_remove_liquidity`: validar entradas → `redeem_allocation_floor` (floor) → verificar `MIN_RESERVE` → garantir que ao menos um
  ativo saiu (`InputTooSmall`).

**Casos de borda monitorados**

* *Underflow/Overflow*: todas as conversões `U256 → u128` são guardadas por `u256_to_u128_checked`, retornando `AmmError::Overflow` quando o
  valor real excede o domínio; divisões também retornam `Overflow` se o denominador zerar. No `gross_up_fee` fixamos `hi = 1` quando o ceil
  retornaria zero, evitando laços infinitos.
* *Zeros e mínimos*: `ensure_nonzero` barra `dx`, `dy` e `burn_shares` nulos, e `ensure_reserves` impede reservas abaixo de `MIN_RESERVE` antes
  de qualquer arredondamento. Após calcular `out`, se `y'` cair abaixo do mínimo emitimos `MinReserveBreached` em vez de saturar.
* *Tolerâncias extremas*: `min_out_with_tolerance` aceita `slippage_tolerance_ppm = 1e6` e pode retornar zero explícito; `max_in_with_tolerance`
  erra para cima (`ceil`) e retorna `InputTooSmall` quando a tolerância inviabiliza o denominador `1e6 - fee`.
* *Slippage degenerado*: quando `p_exec ≥ p_spot`, forçamos `slippage_ppm = 0`; caso contrário, o clamp `0..PPM_SCALE` garante que alertas não
  ultrapassem a escala de PPM.

**Alinhamento com notação e fórmulas**

Os símbolos `x`, `y`, `dx`, `dy`, `S`, `fee_ppm`, `tol` e `MIN_RESERVE` seguem exatamente as definições do módulo `src/amm/types.rs`. Assim que
as seções `SECTION:NOTATION` e `SECTION:FORMULAS` do README forem publicadas (Thread A), esta política já está aderente: as etapas descritas
acima espelham os mesmos símbolos e a ordem de cálculo (`fee → invariável → checagens`) usada nas fórmulas do código-fonte.

<!-- END:ROUNDING -->

## Notação

<!-- SECTION:NOTATION -->

Os símbolos adotam o padrão `snake_case` dos parâmetros no código, com prefixos que indicam o domínio (`R_` para reservas, `dx`/`dy` para quantidades trocadas e `S_` para shares). Valores em `Wad` são inteiros de 128 bits escalados por 1e18; parâmetros em `ppm` usam escala fixa de 1e6. Sufixos como `_prime` representam estados pós-transformação, enquanto `_eff` indica valores já clampados.

| Símbolo     | Nome                                   | Unidade | Escala/Decimais | Tipo       | Fonte (arquivo:linha)   |
| ----------- | -------------------------------------- | ------- | --------------- | ---------- | ----------------------- |
| R_x         | Reserva do ativo X                     | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:42      |
| R_y         | Reserva do ativo Y                     | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:42      |
| dx_gross    | Quantidade bruta enviada de X          | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:42      |
| f_ppm       | Taxa aplicada em partes por milhão     | ppm     | 1e6 (6 casas)   | u32        | src/amm/swap.rs:42      |
| dx_fee      | Taxa cobrada sobre dx_gross            | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:48      |
| dx_net      | Quantidade líquida após taxa           | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:49      |
| R_x_prime   | Reserva X pós-entrada                  | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:55      |
| k           | Invariante produto R_x * R_y           | Wad²    | 1e36 (36 casas) | U256       | src/amm/swap.rs:58      |
| R_y_star    | Reserva Y hipotética pós-troca         | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:59      |
| dy_out      | Quantidade de Y entregue na troca      | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:62      |
| dy_target   | Quantidade alvo de Y desejada          | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:82      |
| dx_upper    | Limite superior bruto usado na busca   | Wad     | 1e18 (18 casas) | u128       | src/amm/swap.rs:105     |
| dx_core     | dx calculado por get_amount_in         | Wad     | 1e18 (18 casas) | u128       | src/amm/pricing.rs:101  |
| p_spot_xy   | Preço spot de X em Y                   | Wad     | 1e18 (18 casas) | u128       | src/amm/pricing.rs:19   |
| p_spot_yx   | Preço spot de Y em X                   | Wad     | 1e18 (18 casas) | u128       | src/amm/pricing.rs:26   |
| p_exec_xy   | Preço efetivo da troca X→Y             | Wad     | 1e18 (18 casas) | u128       | src/amm/pricing.rs:34   |
| slip_ppm    | Slippage relativo em ppm               | ppm     | 1e6 (6 casas)   | u32        | src/amm/pricing.rs:45   |
| tol_ppm     | Tolerância de slippage informada       | ppm     | 1e6 (6 casas)   | u32        | src/amm/pricing.rs:75   |
| tol_eff     | Tolerância após clamp 0..PPM_SCALE     | ppm     | 1e6 (6 casas)   | u64        | src/amm/pricing.rs:79   |
| dy_min      | Mínimo aceitável de Y com tolerância   | Wad     | 1e18 (18 casas) | u128       | src/amm/pricing.rs:84   |
| dx_max      | Máximo aceitável de X com tolerância   | Wad     | 1e18 (18 casas) | u128       | src/amm/pricing.rs:108  |
| S_tot       | Total de shares em circulação          | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:44 |
| dx_add      | Quantidade de X adicionada na liquidez | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:44 |
| dy_add      | Quantidade de Y adicionada na liquidez | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:44 |
| shares_mint | Shares emitidas para o provedor        | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:54 |
| S_burn      | Shares queimadas no resgate            | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:66 |
| x_withdraw  | Quantidade de X retirada no burn       | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:74 |
| y_withdraw  | Quantidade de Y retirada no burn       | Wad     | 1e18 (18 casas) | u128       | src/amm/liquidity.rs:75 |
| WAD         | Escala fixa para valores (1e18)        | Wad     | 1e18 (18 casas) | const u128 | src/amm/types.rs:15     |
| PPM_SCALE   | Escala fixa para ppm                   | ppm     | 1e6 (6 casas)   | const u32  | src/amm/types.rs:16     |
| MIN_RESERVE | Reserva mínima por ativo               | Wad     | 1e18 (18 casas) | const u128 | src/amm/types.rs:17     |

<!-- END:NOTATION -->

## Exemplos (Didático / Realista)

<!-- SECTION:EXAMPLES -->

### Exemplos Didáticos

<!-- SUBSECTION:DIDATIC -->

#### EX1: Cálculo de amount_out (X→Y)

**Contexto:** demonstra a sequência completa de `quote_in`, cobrando taxa de entrada com `ceil`, resolvendo o invariável com `nearest-even` e finalizando o `amount_out` com `floor`.

**Entradas (unidades/escala entre parênteses):**

|  Parâmetro |          Valor | Unidade/Escala |
| ---------: | -------------: | :------------- |
|      `R_x` | `50_000 * WAD` | WAD (1e18)     |
|      `R_y` | `80_000 * WAD` | WAD (1e18)     |
| `dx_gross` |  `1_234 * WAD` | WAD (1e18)     |
|    `f_ppm` |        `3_000` | ppm (1e6)      |

**Passos (com fórmulas ASCII):**

1. Cobrar taxa sobre `dx_gross` — `dx_fee = ceil(dx_gross * f_ppm / PPM_SCALE)` ⇒ **3.702000000000 WAD**
   *Rounding:* ceil em apply_input_fee — ver `rounding_matrix`: `swap_get_amount_out/apply_input_fee`
2. Apurar entrada líquida — `dx_net = dx_gross - dx_fee` ⇒ **1_230.298000000000 WAD**
   *Rounding:* n/a (subtração inteira)
3. Resolver o invariável — `y_star = round_nearest_even((R_x * R_y) / (R_x + dx_net))` ⇒ **78_078.796262321176 WAD**
   *Rounding:* bankers em solve_invariant — ver `rounding_matrix`: `swap_get_amount_out/solve_invariant`
4. Finalizar o `amount_out` — `dy_out = R_y - y_star` ⇒ **1_921.203737678824 WAD**
   *Rounding:* floor em finalize_out_amount — ver `rounding_matrix`: `swap_get_amount_out/finalize_out_amount`
5. Validar reserva mínima — `R_y' = R_y - dy_out = 78_078.796262321176 WAD ≥ MIN_RESERVE`
   *Rounding:* n/a (checagem de limite)

**Resultado:** `dy_out` = **1_921.203737678824 WAD**

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{swap, types::WAD};

let out = swap::get_amount_out(50_000 * WAD, 80_000 * WAD, 1_234 * WAD, 3_000).unwrap();
assert_eq!(out, 1_921_203_737_678_824_355_072u128);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Cálculo de amount_out (X→Y)); Rounding → § *Política de Rounding* (`swap_get_amount_out/apply_input_fee`, `swap_get_amount_out/solve_invariant`, `swap_get_amount_out/finalize_out_amount`)

#### EX2: Amount_in mínimo para alvo Y

**Contexto:** ilustra `quote_out`, destacando os dois `ceil` sucessivos que garantem `dx` suficiente mesmo com taxa positiva.

**Entradas (unidades/escala entre parênteses):**

|   Parâmetro |          Valor | Unidade/Escala |
| ----------: | -------------: | :------------- |
|       `R_x` | `50_000 * WAD` | WAD (1e18)     |
|       `R_y` | `80_000 * WAD` | WAD (1e18)     |
| `dy_target` |  `1_850 * WAD` | WAD (1e18)     |
|     `f_ppm` |        `3_000` | ppm (1e6)      |

**Passos (com fórmulas ASCII):**

1. Calcular o alvo líquido — `dx_net = ceil(R_x * dy_target / (R_y - dy_target))` ⇒ **1_183.621241202815 WAD**
   *Rounding:* ceil em compute_net_target — ver `rounding_matrix`: `swap_get_amount_in/compute_net_target`
2. Fazer o gross-up da taxa — `dx_hi = ceil(dx_net * PPM_SCALE / (PPM_SCALE - f_ppm))` ⇒ **1_187.182789571530 WAD**
   *Rounding:* ceil em gross_up_fee — ver `rounding_matrix`: `swap_get_amount_in/gross_up_fee`
3. Busca binária minimal — `dx_final = min { dx | get_amount_out(R_x, R_y, dx, f_ppm) ≥ dy_target }` ⇒ **1_187.182789571530 WAD**
   *Rounding:* n/a (busca discreta)

**Resultado:** `dx_final` = **1_187.182789571530 WAD**

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{swap, types::WAD};

let dx = swap::get_amount_in(50_000 * WAD, 80_000 * WAD, 1_850 * WAD, 3_000).unwrap();
assert_eq!(dx, 1_187_182_789_571_529_688_233u128);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Amount_in mínimo para alvo Y); Rounding → § *Política de Rounding* (`swap_get_amount_in/compute_net_target`, `swap_get_amount_in/gross_up_fee`)

#### EX3: Slippage relativo X→Y

**Contexto:** cobre a cadeia `spot → execution → slippage`, evidenciando o uso de `nearest-even` e o clamp final para ppm.

**Entradas (unidades/escala entre parênteses):**

|  Parâmetro |          Valor | Unidade/Escala |
| ---------: | -------------: | :------------- |
|      `R_x` | `50_000 * WAD` | WAD (1e18)     |
|      `R_y` | `80_000 * WAD` | WAD (1e18)     |
| `dx_gross` |  `1_234 * WAD` | WAD (1e18)     |
|    `f_ppm` |        `3_000` | ppm (1e6)      |

**Passos (com fórmulas ASCII):**

1. Spot price instantâneo — `p_spot = round_nearest_even(R_y * WAD / R_x)` ⇒ **1.600000000000 WAD**
   *Rounding:* bankers em compute_spot_price — ver `rounding_matrix`: `pricing_spot_price_x_in_y/compute_spot_price`
2. Preço efetivo observado — `p_exec = round_nearest_even(get_amount_out(R_x, R_y, dx_gross, f_ppm) * WAD / dx_gross)` ⇒ **1.556891197471 WAD**
   *Rounding:* bankers em compute_execution_price — ver `rounding_matrix`: `pricing_execution_price_x_to_y/compute_execution_price`
3. Normalizar o slippage — `slip_raw = round_nearest_even((p_spot - p_exec) * PPM_SCALE / p_spot)` ⇒ **26_943 ppm**
   *Rounding:* bankers em normalize_slippage_ratio — ver `rounding_matrix`: `pricing_slippage_ppm_x_to_y/normalize_slippage_ratio`
4. Aplicar limites — `slippage_ppm = min(slip_raw, PPM_SCALE)` ⇒ **26_943 ppm**
   *Rounding:* none em clamp_slippage_bounds — ver `rounding_matrix`: `pricing_slippage_ppm_x_to_y/clamp_slippage_bounds`

**Resultado:** `slippage_ppm` = **26_943 ppm**

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{pricing, types::WAD};

let ppm = pricing::slippage_ppm_x_to_y(50_000 * WAD, 80_000 * WAD, 1_234 * WAD, 3_000).unwrap();
assert_eq!(ppm, 26_943u32);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Slippage relativo X→Y); Rounding → § *Política de Rounding* (`pricing_spot_price_x_in_y/compute_spot_price`, `pricing_execution_price_x_to_y/compute_execution_price`, `pricing_slippage_ppm_x_to_y/normalize_slippage_ratio`, `pricing_slippage_ppm_x_to_y/clamp_slippage_bounds`)

#### EX4: Add liquidity proporcional

**Contexto:** demonstra a alocação proporcional que usa `floor` para limitar o mint ao braço mais curto.

**Entradas (unidades/escala entre parênteses):**

| Parâmetro |           Valor | Unidade/Escala |
| --------: | --------------: | :------------- |
|     `R_x` | `120_000 * WAD` | WAD (1e18)     |
|     `R_y` |  `75_000 * WAD` | WAD (1e18)     |
|  `dx_add` |   `1_000 * WAD` | WAD (1e18)     |
|  `dy_add` |     `450 * WAD` | WAD (1e18)     |
|   `S_tot` |  `50_000 * WAD` | WAD (1e18)     |

**Passos (com fórmulas ASCII):**

1. Projetar cada braço — `shares_x = floor(dx_add * S_tot / R_x)`, `shares_y = floor(dy_add * S_tot / R_y)` ⇒ **416.666666666666 WAD** e **300.000000000000 WAD**
   *Rounding:* floor em proportional_allocation_floor — ver `rounding_matrix`: `liquidity_add_liquidity/proportional_allocation_floor`
2. Escolher o limitante — `shares_mint = min(shares_x, shares_y)` ⇒ **300.000000000000 WAD**
   *Rounding:* n/a (mínimo discreto)
3. Pós-condição — `R_x' = R_x + dx_add`, `R_y' = R_y + dy_add` (ambos ≥ `MIN_RESERVE`)
   *Rounding:* n/a (somas inteiras)

**Resultado:** `shares_mint` = **300.000000000000 WAD**

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{liquidity, types::WAD};

let shares = liquidity::add_liquidity(120_000 * WAD, 75_000 * WAD, 1_000 * WAD, 450 * WAD, 50_000 * WAD).unwrap();
assert_eq!(shares, 300 * WAD);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Add liquidity proporcional); Rounding → § *Política de Rounding* (`liquidity_add_liquidity/proportional_allocation_floor`)

<!-- END:SUBSECTION:DIDATIC -->

### Exemplos Realistas

<!-- SUBSECTION:REALISTIC -->

#### RX1: Cálculo de amount_out (X→Y) — swap VOL→STBL com fee de 30 bps

**Contexto realista:** pool VOL/STBL com reservas profundas (`125M` VOL vs. `83M` STBL). O livro da corretora mantém STBL com 6 casas, mas o roteador normaliza para WAD (1e18) antes da cotação. A mesa envia `275k` VOL com taxa de taker de 30 bps para testar a fronteira de rounding da cobrança de fee.

**Parâmetros (unidades/escala entre parênteses):**

|     Parâmetro |                                                   Valor | Unidade/Escala   |
| ------------: | ------------------------------------------------------: | :--------------- |
|    decimals_V |                                                      18 | casas            |
| decimals_STBL |                                6 → normalizado para WAD | casas            |
|      reserves | (R_V, R_STBL) = (125_000_000.000000, 83_000_000.000000) | unidades nativas |
|       fee_bps |                                                      30 | bps              |
|         input |                                 dx = 275_000.432109 VOL | WAD (1e18)       |

**Passos (com fórmulas ASCII e rounding):**

1. Aplica fee sobre o input — `dx_fee = ceil(dx * f_ppm / PPM_SCALE)` ⇒ **825.001296327 VOL**
   *Rounding:* ceil em `apply_input_fee` — ver `rounding_matrix`: `swap_get_amount_out/apply_input_fee` (ref `src/amm/swap.rs:27..35`)
2. Net do input — `dx_net = dx - dx_fee` ⇒ **274_175.430812673 VOL**
3. Atualiza reserva X — `R_V' = R_V + dx_net` ⇒ **125_274_175.430812673 VOL**
4. Resolve o invariável — `R_STBL* = round_nearest_even((R_V * R_STBL) / R_V')` ⇒ **82_818_345.954549746632789137 STBL**
   *Rounding:* bankers em `solve_invariant` — ver `rounding_matrix`: `swap_get_amount_out/solve_invariant` (ref `src/amm/swap.rs:57..59`)
5. Diferencial de saída — `dy_out = R_STBL - R_STBL*` ⇒ **181_654.045450253367210863 STBL** e `R_STBL' = 82_818_345.954549746632789137 STBL`
   *Rounding:* floor em `finalize_out_amount` — ver `rounding_matrix`: `swap_get_amount_out/finalize_out_amount` (ref `src/amm/swap.rs:61..68`)

**Resultado:** `dy_out` = **181_654.045450253367210863 STBL** (WAD, 18 casas)

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{swap, types::WAD};

let x = 125_000_000u128 * WAD;
let y = 83_000_000u128 * WAD;
let dx = 275_000_432_109_000_000_000_000u128; // 275000.432109 VOL
let dy_out = swap::get_amount_out(x, y, dx, 3_000).unwrap();
assert_eq!(dy_out, 181_654_045_450_253_367_210_863u128);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Cálculo de amount_out (X→Y)); Rounding → § *Política de Rounding* (`swap_get_amount_out/apply_input_fee`, `swap_get_amount_out/solve_invariant`, `swap_get_amount_out/finalize_out_amount`)

#### RX2: Amount_in mínimo para alvo Y — swap VOL→STBL próximo ao limite de reserva

**Contexto realista:** carteira de crédito precisa sacar `525.500875012` STBL de um pool com apenas `12.1M` STBL líquidos (já normalizados para WAD). O pedido respeita o mínimo de reserva (`R_STBL - dy ≈ 12M`), mas pressiona o rounding de ceil nas estimativas para garantir que o protocolo capture taxa integral (50 bps).

**Parâmetros (unidades/escala entre parênteses):**

|     Parâmetro |                                                  Valor | Unidade/Escala   |
| ------------: | -----------------------------------------------------: | :--------------- |
|    decimals_V |                                                     18 | casas            |
| decimals_STBL |                               6 → normalizado para WAD | casas            |
|      reserves | (R_V, R_STBL) = (48_000_000.000000, 12_100_000.000000) | unidades nativas |
|       fee_bps |                                                     50 | bps              |
|         input |                        dy_target = 525_500.875012 STBL | WAD (1e18)       |

**Passos (com fórmulas ASCII e rounding):**

1. Checagem de segurança — `R_STBL - dy_target = 12_099_999.474499125 STBL` (mantém ≥ `MIN_RESERVE`)
2. Estima net — `dx_net_est = ceil(R_V * dy_target / (R_STBL - dy_target))` ⇒ **2_179_277.196204561579795744 VOL**
   *Rounding:* ceil em `compute_net_target` — ver `rounding_matrix`: `swap_get_amount_in/compute_net_target` (ref `src/amm/swap.rs:94..96`)
3. Faz gross-up da taxa — `dx_gross_guess = ceil(dx_net_est * PPM_SCALE / (PPM_SCALE - f_ppm))` ⇒ **2_190_228.337894031738488185 VOL**
   *Rounding:* ceil em `gross_up_fee` — ver `rounding_matrix`: `swap_get_amount_in/gross_up_fee` (ref `src/amm/swap.rs:105..111`)
4. Busca binária com `get_amount_out` ⇒ menor `dx_in = 2_190_228.337894031738488183 VOL` que entrega `dy_target` respeitando os roundings de RX1

**Resultado:** `dx_in` = **2_190_228.337894031738488183 VOL** (WAD, 18 casas)

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{swap, types::WAD};

let x = 48_000_000u128 * WAD;
let y = 12_100_000u128 * WAD;
let dy = 525_500_875_012_000_000_000_000u128;
let dx = swap::get_amount_in(x, y, dy, 5_000).unwrap();
assert_eq!(dx, 2_190_228_337_894_031_738_488_183u128);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Amount_in mínimo para alvo Y); Rounding → § *Política de Rounding* (`swap_get_amount_in/compute_net_target`, `swap_get_amount_in/gross_up_fee`, `swap_get_amount_out/*`)

#### RX3: Min out com tolerância — trade de alta tolerância (95%) para VOL→STBL

**Contexto realista:** um agregador móvel aceita slippage de até 95% para garantir execução num cenário de stress de liquidez. As reservas são assimétricas (`90.5M` VOL vs. `64.25M` STBL), e a operação demonstra o rounding floor ao aplicar o desconto de tolerância.

**Parâmetros (unidades/escala entre parênteses):**

|              Parâmetro |                                                  Valor | Unidade/Escala   |
| ---------------------: | -----------------------------------------------------: | :--------------- |
|             decimals_V |                                                     18 | casas            |
|          decimals_STBL |                               6 → normalizado para WAD | casas            |
|               reserves | (R_V, R_STBL) = (90_500_000.000000, 64_250_000.000000) | unidades nativas |
|                fee_bps |                                                     30 | bps              |
| slippage_tolerance_ppm |                                                950_000 | ppm              |
|                  input |                              dx = 1_200_000.125987 VOL | WAD (1e18)       |

**Passos (com fórmulas ASCII e rounding):**

1. Cotação bruta — `dy_out = get_amount_out(R_V, R_STBL, dx, fee_ppm)` ⇒ **838_295.810577985881513049 STBL** (mesmos roundings de RX1)
2. Clamp da tolerância — `tol_eff = min(950_000, PPM_SCALE) = 950_000 ppm`
3. Desconto conservador — `dy_min = floor(dy_out * (PPM_SCALE - tol_eff) / PPM_SCALE)` ⇒ **41_914.790528899294075652 STBL**
   *Rounding:* floor em `apply_slippage_discount` — ver `rounding_matrix`: `pricing_min_out_with_tolerance/apply_slippage_discount` (ref `src/amm/pricing.rs:84..88`)

**Resultado:** `dy_min` = **41_914.790528899294075652 STBL** (WAD, 18 casas)

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{pricing, types::WAD};

let x = 90_500_000u128 * WAD;
let y = 64_250_000u128 * WAD;
let dx = 1_200_000_125_987_000_000_000_000u128;
let dy_min = pricing::min_out_with_tolerance(x, y, dx, 3_000, 950_000).unwrap();
assert_eq!(dy_min, 41_914_790_528_899_294_075_652u128);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Min out com tolerância); Rounding → § *Política de Rounding* (`pricing_min_out_with_tolerance/apply_slippage_discount`, `swap_get_amount_out/*`)

#### RX4: Slippage relativo X→Y — impacto de trade grande com clamp em ppm

**Contexto realista:** roteador institucional avalia a perda de preço efetivo ao vender `950k` VOL num pool desequilibrado (`32.75M` VOL vs. `112.9M` STBL). O cálculo usa preços em WAD e entrega `31_023 ppm` de slippage, cobrindo dois arredondamentos `bankers` e o clamp sem rounding.

**Parâmetros (unidades/escala entre parênteses):**

|     Parâmetro |                                                   Valor | Unidade/Escala   |
| ------------: | ------------------------------------------------------: | :--------------- |
|    decimals_V |                                                      18 | casas            |
| decimals_STBL |                                6 → normalizado para WAD | casas            |
|      reserves | (R_V, R_STBL) = (32_750_000.000000, 112_900_000.000000) | unidades nativas |
|       fee_bps |                                                      30 | bps              |
|         input |                                 dx = 950_000.784321 VOL | WAD (1e18)       |

**Passos (com fórmulas ASCII e rounding):**

1. Preço spot — `p_spot = round_nearest_even((R_STBL * WAD) / R_V)` ⇒ **3.447328244274809160 WAD**
   *Rounding:* bankers em `compute_spot_price` — ver `rounding_matrix`: `pricing_spot_price_x_in_y/compute_spot_price` (ref `src/amm/pricing.rs:19..22`)
2. Preço de execução — `p_exec = round_nearest_even((dy_out * WAD) / dx)` ⇒ **3.340380340412448601 WAD**
   *Rounding:* bankers em `compute_execution_price` — ver `rounding_matrix`: `pricing_execution_price_x_to_y/compute_execution_price` (ref `src/amm/pricing.rs:38..40`)
3. Slippage bruta — `raw_ppm = round_nearest_even(((p_spot - p_exec) * PPM_SCALE) / p_spot)` ⇒ **31_023 ppm**
   *Rounding:* bankers em `normalize_slippage_ratio` — ver `rounding_matrix`: `pricing_slippage_ppm_x_to_y/normalize_slippage_ratio` (ref `src/amm/pricing.rs:52..60`)
4. Clamp final — `slippage_ppm = min(raw_ppm, PPM_SCALE)` ⇒ **31_023 ppm**
   *Rounding:* none em `clamp_slippage_bounds` — ver `rounding_matrix`: `pricing_slippage_ppm_x_to_y/clamp_slippage_bounds` (ref `src/amm/pricing.rs:61..64`)

**Resultado:** `slippage_ppm` = **31_023 ppm** (ppm, Q32.6)

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{pricing, types::WAD};

let x = 32_750_000u128 * WAD;
let y = 112_900_000u128 * WAD;
let dx = 950_000_784_321_000_000_000_000u128;
let slip = pricing::slippage_ppm_x_to_y(x, y, dx, 3_000).unwrap();
assert_eq!(slip, 31_023u32);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Slippage relativo X→Y); Rounding → § *Política de Rounding* (`pricing_spot_price_x_in_y/compute_spot_price`, `pricing_execution_price_x_to_y/compute_execution_price`, `pricing_slippage_ppm_x_to_y/*`)

#### RX5: Add liquidity proporcional — LP institucional em pool profundo

**Contexto realista:** um provedor adiciona `2.5M` VOL e `2.95M` STBL a um pool com `152.5M` shares em circulação. Ambos os ativos têm 18 casas após normalização, e a alocação é limitada pelo braço de VOL; o exemplo evidencia o floor proporcional usado para proteger LPs contra diluição.

**Parâmetros (unidades/escala entre parênteses):**

|     Parâmetro |                                                    Valor | Unidade/Escala   |
| ------------: | -------------------------------------------------------: | :--------------- |
|    decimals_V |                                                       18 | casas            |
| decimals_STBL |                                                       18 | casas            |
|      reserves |   (R_V, R_STBL) = (78_500_000.000000, 91_500_000.000000) | unidades nativas |
|       fee_bps |                                                        0 | bps              |
|  total_shares |                                152_500_000.000000 shares | WAD (1e18)       |
|         input | (dx, dy) = (2_500_000.458765 VOL, 2_950_000.876543 STBL) | WAD (1e18)       |

**Passos (com fórmulas ASCII e rounding):**

1. Projeção pelo braço de X — `minted_x = floor(dx * S_tot / R_V)` ⇒ **4_856_688.789320541401273885 shares**
   *Rounding:* floor em `proportional_allocation_floor` — ver `rounding_matrix`: `liquidity_add_liquidity/proportional_allocation_floor` (ref `src/amm/liquidity.rs:50..55`)
2. Projeção pelo braço de Y — `minted_y = floor(dy * S_tot / R_STBL)` ⇒ **4_916_668.127571666666666666 shares** (mesmo estágio de rounding)
3. Escolha do mínimo — `shares_minted = min(minted_x, minted_y)` ⇒ **4_856_688.789320541401273885 shares**

**Resultado:** `shares_minted` = **4_856_688.789320541401273885 shares** (WAD, 18 casas)

**Verificação (snippet executável):**

```rust
use credit_engine_core::amm::{liquidity, types::WAD};

let x = 78_500_000u128 * WAD;
let y = 91_500_000u128 * WAD;
let total_shares = 152_500_000u128 * WAD;
let dx_add = 2_500_000_458_765_000_000_000_000u128;
let dy_add = 2_950_000_876_543_000_000_000_000u128;
let minted = liquidity::add_liquidity(x, y, dx_add, dy_add, total_shares).unwrap();
assert_eq!(minted, 4_856_688_789_320_541_401_273_885u128);
```

**Referências:** Fórmulas → § *Fórmulas do Módulo* (Add liquidity proporcional); Rounding → § *Política de Rounding* (`liquidity_add_liquidity/proportional_allocation_floor`)

**Cobertura complementar (IDs adicionais):**

* `liquidity_initial_mint` — validado pelo teste `tests/rounding.rs::r4_mint_is_floor_of_sqrt_xy`, que replica o floor de `√(x·y)` descrito no plano de realismo (`out/docs/examples_realistic_plan.json`).
* `liquidity_remove_liquidity` — exercitado em `tests/rounding.rs::r5_burn_amounts_are_floor_of_proportion`, confirmando os floors proporcionais aplicados nas retiradas do pool.
* `pricing_max_in_with_tolerance` — pode ser reproduzido a partir dos dados de RX2 aplicando o markup `ceil` indicado em § *Fórmulas do Módulo* (Max in com tolerância) e conferindo o estágio `pricing_max_in_with_tolerance/apply_slippage_markup` em `out/docs/rounding_matrix.csv`.
* `pricing_spot_price_y_in_x` — resulta da mesma chamada de preço do cenário RX4 invertendo os papéis de X/Y; o cálculo `pricing::spot_price_y_in_x` com `(R_V, R_STBL)` do exemplo devolve o recíproco em WAD e é listado em `out/docs/examples_realistic_set.csv`.

<!-- END:SUBSECTION:REALISTIC -->

<!-- END:EXAMPLES -->

## Fórmulas do Módulo

<!-- SECTION:FORMULAS -->

### Cálculo de amount_out (X→Y)

**Fórmula (ASCII):** `dx_fee = ceil(dx_gross * f_ppm / PPM_SCALE); dx_net = dx_gross - dx_fee; R_x_prime = R_x + dx_net; R_y_star = round_nearest_even((R_x * R_y) / R_x_prime); dy_out = R_y - R_y_star`

**Entradas:** `R_x, R_y, dx_gross, f_ppm`

**Saídas:** `dy_out`

**Onde no código:** `src/amm/swap.rs:42..71`

**Notas:** Valida reservas mínimas e `dx_gross > 0`, aplica taxa com arredondamento para cima e rejeita saídas que fariam `R_y` cair abaixo de `MIN_RESERVE`.

### Amount_in mínimo para alvo Y

**Fórmula (ASCII):** `dx_net_est = ceil(R_x * dy_target / (R_y - dy_target)); dx_upper = ceil(dx_net_est * PPM_SCALE / (PPM_SCALE - f_ppm)); dx_gross = argmin_dx>=1 get_amount_out(R_x, R_y, dx, f_ppm) ≥ dy_target`

**Entradas:** `R_x, R_y, dy_target, f_ppm`

**Saídas:** `dx_gross`

**Onde no código:** `src/amm/swap.rs:82..167`

**Notas:** Garante `dy_target <= R_y - MIN_RESERVE`, expande `dx_upper` dobrando até cobrir o alvo e executa busca binária para devolver o menor `dx_gross` viável.

### Spot price X em Y

**Fórmula (ASCII):** `p_spot_xy = round_nearest_even((R_y * WAD) / R_x)`

**Entradas:** `R_x, R_y`

**Saídas:** `p_spot_xy`

**Onde no código:** `src/amm/pricing.rs:19..23`

**Notas:** Usa `ensure_reserves` e escala o quociente para WAD antes do arredondamento `nearest-even`.

### Spot price Y em X

**Fórmula (ASCII):** `p_spot_yx = round_nearest_even((R_x * WAD) / R_y)`

**Entradas:** `R_x, R_y`

**Saídas:** `p_spot_yx`

**Onde no código:** `src/amm/pricing.rs:26..30`

**Notas:** Reaproveita a validação de reservas e apenas inverte a razão entre os saldos.

### Preço de execução X→Y

**Fórmula (ASCII):** `dy_out = get_amount_out(R_x, R_y, dx_gross, f_ppm); p_exec_xy = round_nearest_even((dy_out * WAD) / dx_gross)`

**Entradas:** `R_x, R_y, dx_gross, f_ppm`

**Saídas:** `p_exec_xy`

**Onde no código:** `src/amm/pricing.rs:34..40`

**Notas:** Propaga os guardrails de `get_amount_out` e converte o resultado bruto em preço efetivo na mesma escala WAD.

### Slippage relativo X→Y

**Fórmula (ASCII):** `p_spot_xy = spot_price_x_in_y(R_x, R_y); p_exec_xy = execution_price_x_to_y(R_x, R_y, dx_gross, f_ppm); slip_ppm = clamp_0_ppm(round_nearest_even(((p_spot_xy - p_exec_xy) * PPM_SCALE) / p_spot_xy))`

**Entradas:** `R_x, R_y, dx_gross, f_ppm`

**Saídas:** `slip_ppm`

**Onde no código:** `src/amm/pricing.rs:45..65`

**Notas:** Se `p_exec_xy >= p_spot_xy` devolve 0, senão arredonda o delta relativo e aplica clamp em `0..PPM_SCALE` para evitar saturação.

### Min out com tolerância

**Fórmula (ASCII):** `tol_eff = min(tol_ppm, PPM_SCALE); dy_out = get_amount_out(R_x, R_y, dx_gross, f_ppm); dy_min = floor(dy_out * (PPM_SCALE - tol_eff) / PPM_SCALE)`

**Entradas:** `R_x, R_y, dx_gross, f_ppm, tol_ppm`

**Saídas:** `dy_min`

**Onde no código:** `src/amm/pricing.rs:70..88`

**Notas:** Clampa a tolerância informada antes de aplicar o fator `(1 - tol)` e usa divisão inteira para manter arredondamento para baixo.

### Max in com tolerância

**Fórmula (ASCII):** `tol_eff = min(tol_ppm, PPM_SCALE); dx_core = get_amount_in(R_x, R_y, dy_target, f_ppm); dx_max = ceil(dx_core * (PPM_SCALE + tol_eff) / PPM_SCALE)`

**Entradas:** `R_x, R_y, dy_target, f_ppm, tol_ppm`

**Saídas:** `dx_max`

**Onde no código:** `src/amm/pricing.rs:93..112`

**Notas:** Usa o `dx_core` minimal e infla pelo fator `(1 + tol)` com arredondamento para cima para evitar underfill.

### Mint inicial de shares

**Fórmula (ASCII):** `shares_mint = floor(sqrt(R_x * R_y))`

**Entradas:** `R_x, R_y`

**Saídas:** `shares_mint`

**Onde no código:** `src/amm/liquidity.rs:33..39`

**Notas:** Exige reservas válidas (`>= MIN_RESERVE`) e rejeita resultados zerados após a raiz inteira.

### Add liquidity proporcional

**Fórmula (ASCII):** `ratio_x = dx_add * S_tot / R_x; ratio_y = dy_add * S_tot / R_y; shares_mint = floor(min(ratio_x, ratio_y))`

**Entradas:** `R_x, R_y, dx_add, dy_add, S_tot`

**Saídas:** `shares_mint`

**Onde no código:** `src/amm/liquidity.rs:44..61`

**Notas:** Requer `dx_add > 0` e `dy_add > 0`, calcula as proporções por ativo e mantém apenas o menor para preservar a proporção do pool.

### Remove liquidity proporcional

**Fórmula (ASCII):** `x_withdraw = floor(R_x * S_burn / S_tot); y_withdraw = floor(R_y * S_burn / S_tot)`

**Entradas:** `R_x, R_y, S_burn, S_tot`

**Saídas:** `x_withdraw, y_withdraw`

**Onde no código:** `src/amm/liquidity.rs:66..83`

**Notas:** Garante `S_burn <= S_tot`, evita underflow nos saldos e verifica que as reservas remanescentes permanecem acima de `MIN_RESERVE`.

<!-- END:FORMULAS -->

## Build, Test & Bench

<!-- SECTION:BTB -->

### Pré-requisitos

* Ferramentas: `rustc 1.89.0`, `cargo 1.89.0` (instalados via [Rustup](https://rustup.rs/)).
* Ambiente: execute os comandos a partir da raiz do repositório. Para suprimir avisos de telemetria durante os testes, aponte `OTEL_EXPORTER_OTLP_ENDPOINT` para um coletor válido ou inicie `docker compose -f docker-compose.observability.yml up -d` para subir o stack de observabilidade (collector + Jaeger + Prometheus + Grafana).

### Build

```bash
cargo build --release
```

**Notas:** gera artefatos otimizados em `target/release/` (binários e `libcredit_engine_core.rlib`). Use `--features` para habilitar recursos opcionais e execute `cargo clean` antes de builds reproduzíveis ou quando alternar toolchains.

### Test

```bash
cargo test --all -- --nocapture
# exemplos de filtros
cargo test swap::tests::t_out_symmetric_with_fee -- --nocapture
```

**Leitura dos resultados:** o sumário do `cargo test` indica contagem de testes por crate (ex.: `43 passed; 0 failed`). Logs adicionais aparecem inline porque usamos `--nocapture`. Caso veja `BatchSpanProcessor.ExportError`, significa apenas que não há um coletor OTLP ouvindo em `http://localhost:4318`; suba o stack observability citado acima ou exporte `OTEL_EXPORTER_OTLP_ENDPOINT` para um endpoint acessível se precisar validar telemetria.

### Bench

```bash
cargo bench
```

**Leitura dos resultados:** o Criterion imprime no console medianas/intervalos (ex.: `swap/sym_small_f0 time: [177 ns 185 ns]`) e grava relatórios HTML/JSON em `target/criterion/<grupo>/<bench>/report/`. Para comparar execuções, salve uma baseline (`cargo bench -- --save-baseline main`) e compare depois (`cargo bench -- --baseline main`).

### FAQ rápido

* Como rodar só os testes do módulo X? → `cargo test module::submodule -- --nocapture` filtra por namespace ou nome de teste.
* Como selecionar um benchmark específico? → `cargo bench swap/sym_small_f0` isola o benchmark desejado e grava os relatórios correspondentes em `target/criterion/swap/sym_small_f0/`.
* Dicas de troubleshooting comuns. → Falhas de build geralmente são resolvidas com `cargo clean` + rebuild; erros de exportação OTLP durante os testes indicam apenas que o coletor não está ativo e podem ser ignorados se o foco for lógica de negócios.

**Notas sobre execuções recentes**

Os guardrails de build e testes automatizados continuam ativos, com logs versionados em `out/logs/`. As execuções mais recentes incluem:

* `cargo check` — compilação completa sem erros em 7,66 segundos (ver `out/logs/cargo_check.log`).
* `cargo clippy` — lint estático finalizado sem warnings, cobrindo crates internos e dependências (`out/logs/cargo_clippy.log`).
* `cargo test --all -- --nocapture` — suite integral (unit, property e os casos `readme_*`) concluída com 63 testes verdes (`out/logs/cargo_test.log`).

Esses comandos alimentam as verificações de consistência entre README e código; ao atualizar exemplos ou fórmulas, execute-os para garantir que os números documentados continuem válidos.

**ADRs & CI (Complemento)**

* `docs/adr/ADR-0001-numeric-model.md` consolida o modelo numérico e a política de arredondamento; `docs/adr/ADR-0002-swap-fee-and-formula.md` detalha a incidência de taxas de swap.
* O pipeline `scripts/a110_run_invariants.sh` integra os watchers de `ops/watchers/` com o mapa de hooks em `ops/hooks/a110.yml`, garantindo que `watchers.dry` e `hooks.dry` rodem antes das suites Rust.
* A mesma cadência de lint/teste descrita acima é replicada nas rotinas de CI, mantendo alinhados os artefatos de documentação (`out/docs/*.csv|json`) e os validadores automatizados.

<!-- END:BTB -->
