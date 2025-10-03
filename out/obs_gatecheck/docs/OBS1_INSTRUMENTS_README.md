# OBS-1 Instrumentos • Guia de Uso (Thread 8)

Esta nota orienta o registro dos instrumentos canônicos de métricas (`telemetry_instruments`).

## Como registrar

```rust
use opentelemetry::metrics::MeterProvider as _;
use credit_engine_core::telemetry_instruments::register_amm_metrics;

let meter = provider.meter("ce-amm");
let amm_metrics = register_amm_metrics(&meter);
// Agora você possui handles para: latency_hist, hook_execs, data_freshness, cdc_lag, drift_score
```

## Regras obrigatórias

1. **Labels controlados**: use apenas `op`, `service`, `env`, `version` nas métricas de latência. Consulte `allowed_labels()`.
2. **Sem PII**: qualquer label contendo PII (e.g. `email`, `cpf`, `phone`, `address`, `name`, `geo`, `person_`) ou terminando em `_uuid`/`_hash` é proibido. `is_label_forbidden()` cobre os casos conhecidos.
3. **Nada de valores fictícios**: a thread não publica métricas de produção. Em testes, registre amostras efêmeras conforme o contrato.
4. **Sem callbacks improvisados**: os gauges observáveis não devem ter callbacks que exponham dados reais até que as threads subsequentes o definam.
5. **Reprodutibilidade**: qualquer alteração nos instrumentos exige atualização simultânea de código, testes, documentação e evidências.

## Anti-padrões proibidos

- Criar instrumentos adicionais (ex.: `*_tmp`, `*_demo`, `latency_test`).
- Usar labels dinâmicos (hashes, UUIDs, IDs de usuário/conta, qualquer identificador pessoal).
- Gravar métricas com payloads de produção antes da liberação do provider oficial (Threads 5/6/9).
- Misturar chaves de labels de dominios distintos sem aprovação (ex.: `feature` em `amm_op_latency_seconds`).

## Evidências

- Contrato: `docs/obs1_instruments_contract.md`
- Implementação: `src/telemetry_instruments.rs`
- Testes: `tests/telemetry_instruments_tests.rs`
- Relatório: `out/obs_gatecheck/evidence/obs1_instruments_report.json`
