# OBS-1 Gatecheck • Helpers de spans AMM & Pricing

Este README orienta squads sobre como usar o módulo `telemetry_spans_amm` no Credit Engine Core, como conectá-lo a um provider de
traces (Thread 4) e como depurar atributos ausentes durante validações de gate OBS-1.

## 1. Uso rápido

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
let quote = in_amm_swap(&attrs, || execute_swap(request));
```

Helpers disponíveis:

- `span_amm_swap` / `in_amm_swap`
- `span_amm_add_liquidity` / `in_amm_add_liquidity`
- `span_amm_remove_liquidity` / `in_amm_remove_liquidity`
- `span_pricing_quote` / `in_pricing_quote`

Todos validam atributos (NaN/Inf, limites mínimos e máximos) e populam os campos obrigatórios:
`amm.k_before`, `amm.k_after`, `amm.delta_k_ratio`, `amm.fee_ppm`, `amm.input`, `amm.output`, `op`.

## 2. Conectando a um tracer provider (Thread 4)

1. Inicialize o provider desejado (ex.: OTLP → collector) em sua aplicação, usando `tracing_subscriber` ou equivalente.
2. Registre o layer/opentelemetry antes de chamar qualquer helper.
   ```rust
   use opentelemetry::sdk::trace as sdktrace;
   use tracing_subscriber::{layer::SubscriberExt, Registry};

   let tracer = opentelemetry_otlp::new_pipeline().tracing().install_simple()?;
   let otel = tracing_opentelemetry::layer().with_tracer(tracer);
   let subscriber = Registry::default().with(otel);
   tracing::subscriber::set_global_default(subscriber)?;
   ```
3. Use os helpers normalmente. Eles **não** instalam subscriber global, garantindo compatibilidade com o bootstrap da Thread 4.

## 3. Troubleshooting

| Sintoma | Checagem | Ação |
| --- | --- | --- |
| Panic `invalid telemetry attributes ...` | Validadores detectaram valor inválido | Corrigir upstream; revisitar cálculos/normalizações |
| Span exportado sem `op` | Span criado manualmente | Migrar para os helpers; teste com `cargo test --features obs --test telemetry_spans_amm_tests` |
| Dashboards/alertas vazios | Filtro depende de `amm.*` | Validar se atributos foram preenchidos; checar `obs1_spans_amm_report.json` |
| Collector sem spans | Subscriber não inicializado | Revise inicialização do provider (Thread 4) e filtros de sampling |

## 4. Validação local

- `cargo test --features obs --test telemetry_spans_amm_tests`
- `rg 'TBD|FIXME|…' -n` (deve retornar vazio)
- Conferir `out/obs_gatecheck/evidence/obs1_spans_amm_report.json`

## 5. Evidências / Gate OBS-1

- `out/obs_gatecheck/evidence/obs1_spans_amm_report.json` contém timestamp, hash SHA256 do módulo e amostra de span exportado.
- Manter README, contrato (`docs/obs1_spans_amm_contract.md`) e evidências sincronizados antes do merge.

---

**Owner:** OBS-1 Telemetria.
