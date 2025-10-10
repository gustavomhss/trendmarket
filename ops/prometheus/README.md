# Recording Rules (OBS-3)

| Record | Window | Interval | Rationale |
| --- | --- | --- | --- |
| `ce:amm_op_latency_seconds:p75` | `rate()[5m]` buckets | 30s | Quantil p75 para acompanhar latência típica por operação/serviço, mantendo buckets alinhados com histogramas de origem. |
| `ce:amm_op_latency_seconds:p95` | `rate()[5m]` buckets | 30s | Quantil p95 para capturar regressões de cauda sem ruído excessivo, usando mesma agregação que o p75. |
| `ce:amm_op_latency_seconds:avg_by_op` | `rate()[5m]` sum/count | 30s | Média compatível com histogramas para checks semânticos e comparações contra contratos. |
| `ce:hook_executions_total:rate5m` | `rate()[5m]` | 30s | Throughput de hooks por `hook_id,status`, suporte direto a SLIs de automação. |
| `ce:data_freshness_seconds:max_by_source` | ponto instantâneo | 30s | Máximo de freshness por fonte para painéis de OBS-10/11 sem explosão de cardinalidade. |
| `ce:cdc_lag_seconds:max_by_stream` | ponto instantâneo | 30s | Maior lag por stream para validar SLAs de CDC e acionar watchers. |
| `ce:drift_score:max_by_feature` | ponto instantâneo | 30s | Valor máximo de drift por feature, destacando riscos em modelos e dados. |

Os quantis usam `sum by (le, op, service)` para preservar os buckets por operação/serviço, evitando perda de fidelidade e mantendo estimativas estáveis mesmo com várias instâncias.

A média (`ce:amm_op_latency_seconds:avg_by_op`) deriva do par `*_sum` e `*_count` com `rate()[5m]`, garantindo coerência com a instrumentação do histograma e evitando discrepâncias com alertas de latência.

Sanidade rápida:
- `promtool check rules ops/prometheus/rules/core.rules.yml`
- Consultar `ce:amm_op_latency_seconds:p95` no console `/graph` para validar séries agregadas.
