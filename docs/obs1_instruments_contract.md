# OBS-1 • Instrumentos Canônicos (Thread 8)

Este documento normativo descreve os instrumentos de métricas registrados pela Thread 8
(`telemetry_instruments`). A finalidade é garantir que clientes do SDK consigam registrar
as métricas de forma determinística, obedecendo o contrato estabelecido na Thread 1.

## 1. Escopo

- Registrar instrumentos canônicos utilizando um `Meter` fornecido pelo chamador.
- Validar e documentar os rótulos (*labels*) permitidos e proibidos.
- Nenhum valor de produção é emitido nesta thread; exemplos são apenas para testes.

## 2. Instrumentos

| Nome                    | Tipo        | Unidade | Descrição                                             | Buckets (se aplicável)                             | Labels        |
|-------------------------|-------------|---------|-------------------------------------------------------|----------------------------------------------------|---------------|
| `amm_op_latency_seconds`| Histograma  | `s`     | Latência por operação AMM em segundos                 | `0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0` | `op`, `service`, `env`, `version` |
| `hook_executions_total` | Counter     | `1`     | Execuções de hooks particionadas por id e status      | _n/a_                                              | `hook_id`, `status` (status ∈ {`success`, `error`}) |
| `data_freshness_seconds`| Gauge (obs) | `s`     | Freshness de dados por fonte e domínio                | _n/a_                                              | `source`, `domain` |
| `cdc_lag_seconds`       | Gauge (obs) | `s`     | Atraso de CDC por stream e partição                   | _n/a_                                              | `stream`, `partition` |
| `drift_score`           | Gauge (obs) | `1`     | Pontuação de drift (esperado 0..1) por feature/domínio| _n/a_                                              | `feature`, `domain` |

> **Observações normativas**
> - O histograma utiliza buckets **fixos** definidos acima; qualquer alteração requer nova thread/contrato.
> - Gauges são apenas registrados. Os callbacks de observação serão definidos em threads futuras.
> - `hook_executions_total` é acumulativo e monotônico, com valores de `status` documentados para integração futura.

## 3. Política de Labels

### 3.1 Permitidos globalmente

A fábrica expõe `allowed_labels()`, que retorna `["op", "service", "env", "version"]`.
Esses rótulos podem ser usados em `amm_op_latency_seconds` e também funcionam como lista segura
para futuros instrumentos derivados.

### 3.2 Proibidos (lista base)

- `user_id`, `account_id`, `request_id`, `session_id`
- Qualquer chave que termine com `_uuid` ou `_hash`
- Qualquer chave que contenha (case-insensitive): `email`, `cpf`, `phone`, `address`, `name`, `geo`, `person_`

A função `is_label_forbidden(k)` aplica os critérios acima. O conjunto é ampliável via contrato,
mas garante que PII ou identificadores de alta cardinalidade não sejam usados.

## 4. API de Registro

```rust
use opentelemetry::metrics::Meter;
use credit_engine_core::telemetry_instruments::register_amm_metrics;

fn setup(meter: &Meter) {
    let instruments = register_amm_metrics(meter);
    // instrumentos disponíveis: latency_hist, hook_execs, data_freshness, cdc_lag, drift_score
}
```

### 4.1 Exemplo efêmero (apenas testes)

```rust
use opentelemetry::KeyValue;

let meter = provider.meter("ce-amm");
let m = register_amm_metrics(&meter);
m.latency_hist.record(
    0.012,
    &[
        KeyValue::new("op", "swap"),
        KeyValue::new("service", "ce-amm"),
        KeyValue::new("env", "dev"),
        KeyValue::new("version", "0.0.0+devhash"),
    ],
);
```

## 5. Regras de Uso

1. **Sem labels dinâmicos**: utilize apenas os rótulos aprovados.
2. **Sem PII**: métricas nunca devem conter identificadores pessoais ou sensíveis.
3. **Sem valores sintéticos em produção**: esta thread não publica métricas reais.
4. **Reprodutibilidade**: qualquer mudança exige atualização simultânea de código, testes, README e evidências.

## 6. Rastros de conformidade

- Código de registro: `src/telemetry_instruments.rs`
- Testes de garantia: `tests/telemetry_instruments_tests.rs`
- Evidência formal: `out/obs_gatecheck/evidence/obs1_instruments_report.json`
