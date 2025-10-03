# OBS-1 Latency Helpers — Guia de Integração

Este README documenta como consumir o módulo `telemetry_latency` em serviços que precisam publicar métricas no histograma `amm_op_latency_seconds`.

## 1. Como injetar um `LatencySink`

A Thread 8/T12 disponibilizará uma implementação conectada ao registry de instrumentos. Enquanto isso, o padrão é expor um sink via *dependency injection*:

```rust
pub struct OtlpSink {
    meter: Arc<dyn HistogramExporter>,
}

impl LatencySink for OtlpSink {
    fn record(&self, seconds: f64, labels: &[Label]) {
        self.meter.observe(seconds, labels);
    }
}
```

Na camada de composição:

```rust
let base_labels = vec![Label::new("service", "amm.core"), Label::new("env", "prod")];
let sink = Arc::new(OtlpSink::new(meter.clone()));
let response = with_latency("swap", &base_labels, sink.as_ref(), || orchestrate_swap(req));
```

## 2. Uso recomendado por camada

| Camada | Sugestão |
|--------|----------|
| Adapters (HTTP/gRPC) | Usar `with_latency` para blocos curtos e retornos imediatos. |
| Core (pricing, swap, CDC) | Usar `guard` no início da função principal para cobrir o fluxo inteiro. |
| Jobs/Workers | Criar um guard por unidade de trabalho (ex.: cada mensagem consumida). |

## 3. Anti-padrões a evitar

- **Labels dinâmicos**: não gere `service`/`version` por request; mantenha valores fixos.
- **Ignorar erros de validação**: trate `LatencyGuard::new` quando estiver em caminhos críticos (ex.: inicialização). Nunca silencie `panic!` sem entender a causa.
- **Cronômetros paralelos**: não misture `std::time::Instant` manual com este módulo; centralize tudo nele para manter consistência.

## 4. Troubleshooting rápido

| Sintoma | Possível causa | Mitigação |
|---------|----------------|-----------|
| Métrica não aparece | Sink não foi injetado ou guard dropado antes do fim | Assegure que o guard viva pelo escopo desejado e o sink esteja registrado. |
| `panic!` na inicialização | Labels inválidos | Valide na partida usando `LatencyGuard::new` e falhe rápido com mensagens claras. |
| Valores zerados | Operações muito curtas (<1µs) | Adicione pequenas `std::thread::yield_now()` ou agregue operações para medições significativas. |

## 5. Próximos passos

- Conectar `LatencySink` à pipeline OTLP (Thread 8/T12).
- Expor dashboards com `amm_op_latency_seconds` segmentado por `service`/`env`.
- Validar a integração via `scripts/obs1_latency_smoke.sh` (opcional nesta thread).
