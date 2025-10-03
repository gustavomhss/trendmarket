# OBS-1 • Contrato de spans para AMM & Pricing (`telemetry_spans_amm`)

Este documento normativo define o contrato canônico dos spans do AMM e de precificação no escopo **OBS-1 / Thread 10**. Ele serve
como referência única para squads de produto, engenharia e SRE manterem telemetria consistente, auditável e alinhada aos packs de
observabilidade (T4/T7/T8/T9).

## 1. Objetivo e abrangência

- Padronizar os nomes de spans e seus atributos obrigatórios para operações de AMM (`swap`, `add_liquidity`, `remove_liquidity`) e
  precificação (`pricing.quote`).
- Fornecer helpers (`span_*` e `in_*`) que encapsulam criação, validação e atribuição dos atributos de telemetria sem acoplar a um
  provider específico (integração com T4 fica a cargo do chamador).
- Validar valores numéricos, evitando NaN/Inf e limites inválidos que possam quebrar dashboards, traces ou gatilhos A110.

## 2. APIs expostas

Todas as APIs residem no módulo `credit_engine_core::telemetry_spans_amm`.

### 2.1 RAII (`span_*`)

| Função | Span | `op` | Struct de entrada |
| --- | --- | --- | --- |
| `span_amm_swap(&SwapAttrs)` | `amm.swap` | `swap` | `SwapAttrs` |
| `span_amm_add_liquidity(&AddLiquidityAttrs)` | `amm.add_liquidity` | `add_liquidity` | `AddLiquidityAttrs` |
| `span_amm_remove_liquidity(&RemoveLiquidityAttrs)` | `amm.remove_liquidity` | `remove_liquidity` | `RemoveLiquidityAttrs` |
| `span_pricing_quote(&PricingQuoteAttrs)` | `pricing.quote` | `pricing` | `PricingQuoteAttrs` |

Cada helper retorna um `tracing::Span` já populado com os atributos obrigatórios. O chamador é responsável por entrar e sair do span
(`let _guard = span.enter();`).

### 2.2 Wrapper (`in_*`)

| Função | Span | Uso |
| --- | --- | --- |
| `in_amm_swap(&SwapAttrs, F)` | `amm.swap` | Executa `F` dentro do span e retorna o resultado |
| `in_amm_add_liquidity(&AddLiquidityAttrs, F)` | `amm.add_liquidity` | Idem |
| `in_amm_remove_liquidity(&RemoveLiquidityAttrs, F)` | `amm.remove_liquidity` | Idem |
| `in_pricing_quote(&PricingQuoteAttrs, F)` | `pricing.quote` | Idem |

As funções wrapper garantem abertura, entrada e fechamento do span automaticamente, mantendo o código de negócio enxuto e
telemetria confiável.

## 3. Estruturas de atributos

Todos os helpers utilizam o mesmo conjunto de campos numéricos para homogeneidade de telemetria.

```rust
pub struct SwapAttrs {
    pub k_before: f64,
    pub k_after: f64,
    pub delta_k_ratio: f64,
    pub fee_ppm: i64,
    pub input: f64,
    pub output: f64,
}
// ... AddLiquidityAttrs, RemoveLiquidityAttrs e PricingQuoteAttrs seguem exatamente a mesma assinatura.
```

## 4. Regras do contrato

### 4.1 Nomes de spans

- `amm.swap`
- `amm.add_liquidity`
- `amm.remove_liquidity`
- `pricing.quote`

### 4.2 Atributos obrigatórios

| Chave | Tipo | Observação |
| --- | --- | --- |
| `amm.k_before` | `f64` | Invariante antes da operação (deve ser > 0) |
| `amm.k_after` | `f64` | Invariante após a operação (deve ser > 0) |
| `amm.delta_k_ratio` | `f64` | Variação relativa (`|valor| ≤ 1e6`) |
| `amm.fee_ppm` | `i64` | Fee em ppm (≥ 0) |
| `amm.input` | `f64` | Quantidade recebida pelo AMM (≥ 0) |
| `amm.output` | `f64` | Quantidade entregue pelo AMM (≥ 0) |
| `op` | `&'static str` | `swap`, `add_liquidity`, `remove_liquidity` ou `pricing` |

### 4.3 Validações

- Nenhum valor pode ser `NaN`, `+∞` ou `-∞`.
- `k_before > 0` e `k_after > 0`.
- `amm.delta_k_ratio.is_finite()` e `|amm.delta_k_ratio| ≤ 1e6`.
- `amm.fee_ppm ≥ 0`.
- `amm.input ≥ 0` e `amm.output ≥ 0`.
- Helpers **panicam** com mensagem clara caso alguma regra seja violada (`invalid telemetry attributes for ...`).

## 5. Exemplos normativos

### 5.1 Span RAII (`amm.swap`)

```rust
use credit_engine_core::telemetry_spans_amm::{span_amm_swap, SwapAttrs};

let attrs = SwapAttrs {
    k_before: 1.0,
    k_after: 1.05,
    delta_k_ratio: 0.05,
    fee_ppm: 300,
    input: 100.0,
    output: 99.7,
};
let span = span_amm_swap(&attrs);
let _enter = span.enter();
// ... código de swap ...
```

### 5.2 Wrapper (`amm.swap`)

```rust
use credit_engine_core::telemetry_spans_amm::{in_amm_swap, SwapAttrs};

let attrs = SwapAttrs {
    k_before: 1.0,
    k_after: 1.05,
    delta_k_ratio: 0.05,
    fee_ppm: 300,
    input: 100.0,
    output: 99.7,
};
let result = in_amm_swap(&attrs, || do_swap());
```

### 5.3 Pricing quote

```rust
use credit_engine_core::telemetry_spans_amm::{in_pricing_quote, PricingQuoteAttrs};

let attrs = PricingQuoteAttrs {
    k_before: 1.0,
    k_after: 1.0,
    delta_k_ratio: 0.0,
    fee_ppm: 120,
    input: 150.0,
    output: 149.94,
};
let price = in_pricing_quote(&attrs, || quote_price(request));
```

## 6. Erros comuns & troubleshooting

| Sintoma | Causa provável | Mitigação |
| --- | --- | --- |
| Panic `invalid telemetry attributes for amm.swap: amm.k_before must be > 0` | Valor de `k_before` ≤ 0 | Corrigir upstream (ex.: snapshot inconsistente do pool) |
| Panic `... must be finite` | Valor `NaN`/`±inf` propagado de cálculo anterior | Normalizar entradas antes de chamar helper |
| Span sem atributo `op` | Span criado manualmente sem usar helpers | Migrar para `telemetry_spans_amm` para cumprir contrato |
| Dashboards vazios | Atributos faltantes rompem filtros | Verificar validações e os testes de contrato (`cargo test --features obs --test telemetry_spans_amm_tests`) |

## 7. Próximos passos (fora do escopo desta thread)

- Integração com providers OTLP/Jaeger (Thread 4).
- Métricas derivadas e watchers automáticos (Threads 8/9).
- Automatizar geração de evidências em pipeline A110.

---

**Owner:** OBS-1 / Produto & Telemetria.

**Última atualização:** (manter alinhado com o hash em `out/obs_gatecheck/evidence/obs1_spans_amm_report.json`).
