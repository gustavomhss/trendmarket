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

