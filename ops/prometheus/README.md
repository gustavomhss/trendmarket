# Tests de Rules (Thread 3)

Execute a suíte com:

```bash
promtool test rules ops/prometheus/tests/core.rules.test.yml
```

Os testes conferem:
- quantis p75 e p95 das regras `ce:amm_op_latency_seconds:*` em cenários saudável e de cauda pesada;
- média derivada de `sum/count` para `op="swap"` sem NaN/Inf;
- throughput positivo dos hooks via `ce:hook_executions_total:rate5m`;
- monotonia dos buckets e coerência entre `*_bucket`, `*_sum` e `*_count` nas séries sintéticas;
- comportamento de cauda pesada garantindo p95 > 2× média sem saturar o maior bucket.

Ao ajustar as rules, alinhe as séries sintéticas (incrementos, buckets e janelas) antes de atualizar os valores esperados para manter as taxas e quantis consistentes.
