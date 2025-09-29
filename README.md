# OpenTelemetry Collector Contrib Distro

This distribution contains all the components from both the [OpenTelemetry Collector](https://github.com/open-telemetry/opentelemetry-collector) repository and the [OpenTelemetry Collector Contrib](https://github.com/open-telemetry/opentelemetry-collector-contrib) repository. This distribution includes open source and vendor supported components.

## Recommendation

As this distribution contains many components, it is a good starting point to try various configurations. However, when running in production, it is recommended to limit the collector to contain only the components necessary for an environment. Some reasons to do this:

* reduce the size of the collector, reducing deployment times for the collector
* improve the security of the collector by reducing the available attack surface area

Building a [custom collector](https://opentelemetry.io/docs/collector/custom-collector/) can be achieved using the [OpenTelemetry Collector Builder](https://github.com/open-telemetry/opentelemetry-collector/tree/main/cmd/builder).

## Components

The full list of components is available in the [manifest](manifest.yaml)

### Rules for Component Inclusion

- Include all extensions at [Alpha stability](https://github.com/open-telemetry/opentelemetry-collector#alpha) or higher and pipeline components that have at least 1 signal at [Alpha stability](https://github.com/open-telemetry/opentelemetry-collector#alpha) or higher.

## Operational Governance: Watchers & Gate A110

This repository now ships with first-class governance artifacts to guarantee that the mandatory watchers and A110 hooks defined in `agents.md` stay green across environments.

### Inventory

- **Watchers:** Domain-specific configurations live under [`ops/watchers/`](ops/watchers). Each `.yml` file describes the KPI, window, action, owner, and rollback policy for the mandatory watches covering DEC, PM, ML, DATA, PLAT, FE, SEC/PRIV and INT domains. These inventories are the single source of truth consumed by `ops/scripts/watchers_dry_run.py`, which now enumerates every `.yml` file in that directory when generating `ops/reports/watchers_dry.json`.
- **Gate A110 hooks:** The consolidated mapping is defined in [`ops/hooks/a110.yml`](ops/hooks/a110.yml). Every watcher is wired to the correct A110 hook, including the required thresholds and evidence links.

### Local enablement

1. Ensure Python 3.11+ is available (the same interpreter used for the existing automation).
2. Run the dry-run validators before coding:

   ```bash
   make watchers.dry
   make hooks.dry
   ```

## Política de Rounding
<!-- SECTION:ROUNDING -->
Os cálculos do módulo de AMM operam em escala fixa (`WAD = 1e18`, `PPM = 1e6`), portanto cada etapa precisa tratar frações de
forma determinística para evitar over-credit aos LPs ou under-collect de taxas. A política adota **ceil** sempre que o protocolo
precisa arrecadar algo (ex.: fee ou tolerância de slippage) e **floor** quando devolve valores ao usuário, enquanto divisões de
preço usam **nearest-even** para permanecerem neutras. Essa combinação garante que a soma das reservas nunca aumente por artefato
de arredondamento e que a experiência da UI permaneça previsível.

Como guardrail adicional, todas as divisões críticas usam `U256` antes do downcast para `u128`, evitando overflow silencioso, e
erro explícito (`AmmError`) substitui qualquer saturação não controlada. Quando uma operação admite um empate, aplicamos
`half-even` para evitar viés sistemático entre as direções de negociação. Os detalhes completos ficam rastreados em
`out/docs/rounding_matrix.csv` e `out/docs/rounding_rules.json`.

| Operação (ID) | Estágio | Base | Decimais | Direção | Momento | Regra de Empate | Racional (econômico/UX) | Referência |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `swap_get_amount_out` | apply_input_fee | Q128.18 | 18 | ceil | before_net_input | n/a | Garante arrecadação mínima da taxa e evita subcobrança percebida pelo trader | `src/amm/swap.rs:27..35` |
| `swap_get_amount_out` | solve_invariant | Q128.18 | 18 | bankers | after_net_input | half-even | Mantém o invariável `k` simétrico e entrega cotação consistente | `src/amm/swap.rs:57..59` |
| `swap_get_amount_out` | finalize_out_amount | Q128.18 | 18 | floor | before_min_reserve_check | n/a | Impede entregar mais Y do que o pool suporta após validar `MIN_RESERVE` | `src/amm/swap.rs:61..68` |
| `swap_get_amount_in` | compute_net_target | Q128.18 | 18 | ceil | before_fee_gross_up | n/a | Garante `dx_net` suficiente para comprar `dy` sem reprocessar buscas | `src/amm/swap.rs:94..96` |
| `swap_get_amount_in` | gross_up_fee | Q128.18 | 18 | ceil | before_search | n/a | Faz o gross-up da taxa e força `hi ≥ 1`, evitando loops infinitos | `src/amm/swap.rs:105..111` |
| `pricing_min_out_with_tolerance` | apply_slippage_discount | Q128.18 | 18 | floor | after_amount_out | n/a | Aplica desconto conservador antes do check de slippage para proteger LPs e usuários | `src/amm/pricing.rs:84..88` |
| `pricing_max_in_with_tolerance` | apply_slippage_markup | Q128.18 | 18 | ceil | after_amount_in | n/a | Gross-up garante margem suficiente para honrar a tolerância informada | `src/amm/pricing.rs:107..110` |
| `pricing_execution_price_x_to_y` | compute_execution_price | Q128.18 | 18 | bankers | after_amount_out | half-even | Preço percebido sem viés direcional mesmo em empates | `src/amm/pricing.rs:38..40` |
| `pricing_spot_price_x_in_y` | compute_spot_price | Q128.18 | 18 | bankers | after_reserve_validation | half-even | Mantém simetria entre pares e evita drift no spot | `src/amm/pricing.rs:19..22` |
| `pricing_spot_price_y_in_x` | compute_spot_price | Q128.18 | 18 | bankers | after_reserve_validation | half-even | Idem acima, invertendo o par | `src/amm/pricing.rs:26..29` |
| `pricing_slippage_ppm_x_to_y` | normalize_slippage_ratio | Q32.6 | 6 | bankers | after_execution_price | half-even | Normaliza `spot` vs `exec` sem viés e sem inflar alertas | `src/amm/pricing.rs:52..60` |
| `pricing_slippage_ppm_x_to_y` | clamp_slippage_bounds | Q32.6 | 6 | none | after_ratio | n/a | Clampa em `0..PPM_SCALE` para evitar overflow e mensagens inválidas | `src/amm/pricing.rs:61..64` |
| `liquidity_initial_mint` | sqrt_liquidity_floor | Q128.18 | 18 | floor | after_invariant_product | n/a | Shares iniciais nunca excedem `√(x·y)` real, protegendo o pool | `src/amm/liquidity.rs:35..39` |
| `liquidity_add_liquidity` | proportional_allocation_floor | Q128.18 | 18 | floor | after_ratio_projection | n/a | Mint limitado pelo braço mais curto para evitar diluição injusta | `src/amm/liquidity.rs:50..55` |
| `liquidity_remove_liquidity` | redeem_allocation_floor | Q128.18 | 18 | floor | before_min_reserve_check | n/a | Saques respeitam `MIN_RESERVE`, evitando underflow de reservas | `src/amm/liquidity.rs:72..83` |

**Ordem do pipeline por operação**

- `swap_get_amount_out`: validar reservas → `apply_input_fee` (ceil) → atualizar `x'` → `solve_invariant` (nearest-even) →
  `finalize_out_amount` (floor) → checar `MIN_RESERVE`.
- `swap_get_amount_in`: validar entradas (`dy`, reservas, fee) → `compute_net_target` (ceil) → `gross_up_fee` (ceil, força `hi ≥ 1`) →
  expandir limite superior → busca binária consumindo `get_amount_out` até `out ≥ dy`.
- `pricing_min_out_with_tolerance`: reusar `get_amount_out` → `apply_slippage_discount` (floor) → devolver `min_out` para UI.
- `pricing_max_in_with_tolerance`: reusar `get_amount_in` → `apply_slippage_markup` (ceil) → devolver `max_in` para UI.
- `pricing_execution_price_x_to_y`: validar entradas → chamar `get_amount_out` → `compute_execution_price` (nearest-even) → exibir WAD.
- `pricing_spot_price_x_in_y` / `pricing_spot_price_y_in_x`: validar reservas → `compute_spot_price` (nearest-even) → publicar WAD.
- `pricing_slippage_ppm_x_to_y`: obter `spot` e `exec` → `normalize_slippage_ratio` (nearest-even) → `clamp_slippage_bounds` (`0..1e6`).
- `liquidity_initial_mint`: validar reservas → calcular `x·y` em `U256` → `sqrt_liquidity_floor` → checar `shares > 0`.
- `liquidity_add_liquidity`: validar entradas → projetar contribuição em cada braço (floor) → `proportional_allocation_floor` (min) → validar
  crescimento das reservas.
- `liquidity_remove_liquidity`: validar entradas → `redeem_allocation_floor` (floor) → verificar `MIN_RESERVE` → garantir que ao menos um
  ativo saiu (`InputTooSmall`).

**Casos de borda monitorados**

- *Underflow/Overflow*: todas as conversões `U256 → u128` são guardadas por `u256_to_u128_checked`, retornando `AmmError::Overflow` quando o
  valor real excede o domínio; divisões também retornam `Overflow` se o denominador zerar. No `gross_up_fee` fixamos `hi = 1` quando o ceil
  retornaria zero, evitando laços infinitos.
- *Zeros e mínimos*: `ensure_nonzero` barra `dx`, `dy` e `burn_shares` nulos, e `ensure_reserves` impede reservas abaixo de `MIN_RESERVE` antes
  de qualquer arredondamento. Após calcular `out`, se `y'` cair abaixo do mínimo emitimos `MinReserveBreached` em vez de saturar.
- *Tolerâncias extremas*: `min_out_with_tolerance` aceita `slippage_tolerance_ppm = 1e6` e pode retornar zero explícito; `max_in_with_tolerance`
  erra para cima (`ceil`) e retorna `InputTooSmall` quando a tolerância inviabiliza o denominador `1e6 - fee`.
- *Slippage degenerado*: quando `p_exec ≥ p_spot`, forçamos `slippage_ppm = 0`; caso contrário, o clamp `0..PPM_SCALE` garante que alertas não
  ultrapassem a escala de PPM.

**Alinhamento com notação e fórmulas**

Os símbolos `x`, `y`, `dx`, `dy`, `S`, `fee_ppm`, `tol` e `MIN_RESERVE` seguem exatamente as definições do módulo `src/amm/types.rs`. Assim que
as seções `SECTION:NOTATION` e `SECTION:FORMULAS` do README forem publicadas (Thread A), esta política já está aderente: as etapas descritas
acima espelham os mesmos símbolos e a ordem de cálculo (`fee → invariável → checagens`) usada nas fórmulas do código-fonte.
<!-- END:ROUNDING -->

   Both commands surface missing watchers, invalid parameters, or mismatched hook bindings with actionable error messages.
3. For a full Gate A110 rehearsal (watchers + hooks + Rust invariants), execute:

   ```bash
   make gate.a110
   ```

   The script aborts immediately if any watcher or hook validation fails, ensuring issues are detected before the Rust suites start.

### CI guidance

- The canonical A110 pipeline (`scripts/a110_run_invariants.sh`) now invokes `watchers.dry` and `hooks.dry` automatically. Any gap in coverage causes the gate to fail with a non-zero exit code, guaranteeing enforcement during pull requests.
- Integrate the Make targets above (or call the scripts directly) in bespoke CI systems to keep behaviour consistent between local, nightly, and production promotion flows.
- When adding a new watcher or hook, commit the YAML changes together with updated documentation so that the dry-run validators remain the single source of truth.
## Notação
<!-- SECTION:NOTATION -->
Os símbolos adotam o padrão `snake_case` dos parâmetros no código, com prefixos que indicam o domínio (`R_` para reservas, `dx`/`dy` para quantidades trocadas e `S_` para shares). Valores em `Wad` são inteiros de 128 bits escalados por 1e18; parâmetros em `ppm` usam escala fixa de 1e6. Sufixos como `_prime` representam estados pós-transformação, enquanto `_eff` indica valores já clampados.

| Símbolo | Nome | Unidade | Escala/Decimais | Tipo | Fonte (arquivo:linha) |
| --- | --- | --- | --- | --- | --- |
| R_x | Reserva do ativo X | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:42 |
| R_y | Reserva do ativo Y | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:42 |
| dx_gross | Quantidade bruta enviada de X | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:42 |
| f_ppm | Taxa aplicada em partes por milhão | ppm | 1e6 (6 casas) | u32 | src/amm/swap.rs:42 |
| dx_fee | Taxa cobrada sobre dx_gross | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:48 |
| dx_net | Quantidade líquida após taxa | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:49 |
| R_x_prime | Reserva X pós-entrada | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:55 |
| k | Invariante produto R_x * R_y | Wad² | 1e36 (36 casas) | U256 | src/amm/swap.rs:58 |
| R_y_star | Reserva Y hipotética pós-troca | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:59 |
| dy_out | Quantidade de Y entregue na troca | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:62 |
| dy_target | Quantidade alvo de Y desejada | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:82 |
| dx_upper | Limite superior bruto usado na busca | Wad | 1e18 (18 casas) | u128 | src/amm/swap.rs:105 |
| dx_core | dx calculado por get_amount_in | Wad | 1e18 (18 casas) | u128 | src/amm/pricing.rs:101 |
| p_spot_xy | Preço spot de X em Y | Wad | 1e18 (18 casas) | u128 | src/amm/pricing.rs:19 |
| p_spot_yx | Preço spot de Y em X | Wad | 1e18 (18 casas) | u128 | src/amm/pricing.rs:26 |
| p_exec_xy | Preço efetivo da troca X→Y | Wad | 1e18 (18 casas) | u128 | src/amm/pricing.rs:34 |
| slip_ppm | Slippage relativo em ppm | ppm | 1e6 (6 casas) | u32 | src/amm/pricing.rs:45 |
| tol_ppm | Tolerância de slippage informada | ppm | 1e6 (6 casas) | u32 | src/amm/pricing.rs:75 |
| tol_eff | Tolerância após clamp 0..PPM_SCALE | ppm | 1e6 (6 casas) | u64 | src/amm/pricing.rs:79 |
| dy_min | Mínimo aceitável de Y com tolerância | Wad | 1e18 (18 casas) | u128 | src/amm/pricing.rs:84 |
| dx_max | Máximo aceitável de X com tolerância | Wad | 1e18 (18 casas) | u128 | src/amm/pricing.rs:108 |
| S_tot | Total de shares em circulação | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:44 |
| dx_add | Quantidade de X adicionada na liquidez | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:44 |
| dy_add | Quantidade de Y adicionada na liquidez | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:44 |
| shares_mint | Shares emitidas para o provedor | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:54 |
| S_burn | Shares queimadas no resgate | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:66 |
| x_withdraw | Quantidade de X retirada no burn | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:74 |
| y_withdraw | Quantidade de Y retirada no burn | Wad | 1e18 (18 casas) | u128 | src/amm/liquidity.rs:75 |
| WAD | Escala fixa para valores (1e18) | Wad | 1e18 (18 casas) | const u128 | src/amm/types.rs:15 |
| PPM_SCALE | Escala fixa para ppm | ppm | 1e6 (6 casas) | const u32 | src/amm/types.rs:16 |
| MIN_RESERVE | Reserva mínima por ativo | Wad | 1e18 (18 casas) | const u128 | src/amm/types.rs:17 |
<!-- END:NOTATION -->

## Fórmulas do Módulo
<!-- SECTION:FORMULAS -->
### Cálculo de amount_out (X→Y)
**Fórmula (ASCII):** `dx_fee = ceil(dx_gross * f_ppm / PPM_SCALE); dx_net = dx_gross - dx_fee; R_x_prime = R_x + dx_net; R_y_star = round_nearest_even((R_x * R_y) / R_x_prime); dy_out = R_y - R_y_star`

**Entradas:** `R_x, R_y, dx_gross, f_ppm`

**Saídas:** `dy_out`

**Onde no código:** `src/amm/swap.rs:42..71`

**Notas:** Valida reservas mínimas e `dx_gross > 0`, aplica taxa com arredondamento para cima e rejeita saídas que fariam `R_y` cair abaixo de `MIN_RESERVE`.

### Amount_in mínimo para alvo Y
**Fórmula (ASCII):** `dx_net_est = ceil(R_x * dy_target / (R_y - dy_target)); dx_upper = ceil(dx_net_est * PPM_SCALE / (PPM_SCALE - f_ppm)); dx_gross = argmin_dx>=1 get_amount_out(R_x, R_y, dx, f_ppm) >= dy_target`

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
**Fórmula (ASCII):** `p_spot_xy = spot_price_x_in_y(...); p_exec_xy = execution_price_x_to_y(...); slip_ppm = clamp_0_ppm(round_nearest_even(((p_spot_xy - p_exec_xy) * PPM_SCALE) / p_spot_xy))`

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
- Ferramentas: `rustc 1.89.0`, `cargo 1.89.0` (instalados via [Rustup](https://rustup.rs/)).
- Ambiente: execute os comandos a partir da raiz do repositório. Para suprimir avisos de telemetria durante os testes, aponte `OTEL_EXPORTER_OTLP_ENDPOINT` para um coletor válido ou inicie `docker compose -f docker-compose.observability.yml up -d` para subir o stack de observabilidade (collector + Jaeger + Prometheus + Grafana).

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
<!-- END:BTB -->
