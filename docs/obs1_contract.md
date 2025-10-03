# OBS-1 Telemetry Contract

**Versão do contrato:** 1.0.0

Este documento define o contrato canônico da thread OBS-1 para métricas, spans e logs estruturados. Ele é a única fonte de verdade consumida pelos demais componentes do projeto CRD-8. Nenhum código pode emitir telemetria fora das convenções aqui descritas.

## 1. Métricas autorizadas

Todas as métricas seguem `snake_case` com sufixo de unidade quando aplicável. Labels válidos obedecem o regex `^[a-z0-9_]{1,32}$`. A lista completa de labels permitidos é `{op, service, env, version, hook_id, status, source, domain, stream, partition, feature}`. É proibido introduzir qualquer label diferente desta lista. Labels proibidos explícitos: `{user_id, account_id, request_id, session_id}` e qualquer chave com sufixo `_uuid` ou `_hash`.

| Nome | Tipo | Unidade | Labels | Buckets | Descrição | Notas de cardinalidade |
| --- | --- | --- | --- | --- | --- | --- |
| `amm_op_latency_seconds` | Histograma | segundos | `op`, `service`, `env`, `version` | `[0.005, 0.01, 0.02, 0.03, 0.05, 0.075, 0.1, 0.15, 0.2, 0.3, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 5.0]` | Latência por operação do AMM, em segundos (por op/versão). | `op` deve usar o mapeamento de spans; `service` fixo em `ce-amm`. | 
| `hook_executions_total` | Counter | n/a | `hook_id`, `status` | n/a | Número de execuções de hooks por id e status. | `status` ∈ {`success`, `error`}. `hook_id` deve ser estável, sem IDs por requisição. |
| `data_freshness_seconds` | Gauge | segundos | `source`, `domain` | n/a | Freshness de dados por fonte e domínio. | Registrar apenas valores reais provenientes dos sistemas de ingestão. |
| `cdc_lag_seconds` | Gauge | segundos | `stream`, `partition` | n/a | Atraso de CDC por stream e partição. | Não utilizar partições dinâmicas sem agregação. |
| `drift_score` | Gauge | adimensional | `feature`, `domain` | n/a | Indicador de drift de features. | Valor esperado entre 0 e 1; nunca usar valores sintéticos nesta fase. |

### 1.1 Exemplo de exposição Prometheus

```
# HELP amm_op_latency_seconds Latência por operação do AMM em segundos
# TYPE amm_op_latency_seconds histogram
amm_op_latency_seconds_bucket{op="swap",service="ce-amm",env="dev",version="1.2.3",le="0.005"} 2
amm_op_latency_seconds_bucket{op="swap",service="ce-amm",env="dev",version="1.2.3",le="0.01"} 5
...
amm_op_latency_seconds_bucket{op="swap",service="ce-amm",env="dev",version="1.2.3",le="5"} 42
amm_op_latency_seconds_sum{op="swap",service="ce-amm",env="dev",version="1.2.3"} 12.34
amm_op_latency_seconds_count{op="swap",service="ce-amm",env="dev",version="1.2.3"} 42
```

### 1.2 Considerações OTLP

* Histograma deve ser reportado como `HistogramDataPoint` com os limites de bucket listados.
* Counter deve usar `Sum` monotônico com `aggregation_temporality = CUMULATIVE`.
* Gauges devem ser reportados como `Gauge` com valores `double` reais. Nunca registrar placeholders.

## 2. Spans e atributos obrigatórios

Spans usam nomes em `dot.notation` e devem incluir os atributos obrigatórios listados. O label de métrica `op` deve coincidir com a coluna "Op" abaixo.

| Span | Atributos obrigatórios (tipo) | Op correspondente |
| --- | --- | --- |
| `amm.swap` | `amm.k_before` (double), `amm.k_after` (double), `amm.delta_k_ratio` (double), `amm.fee_ppm` (integer), `amm.input` (double), `amm.output` (double) | `swap` |
| `amm.add_liquidity` | mesmos atributos acima | `add_liquidity` |
| `amm.remove_liquidity` | mesmos atributos acima | `remove_liquidity` |
| `pricing.quote` | mesmos atributos acima | `pricing` |
| `cdc.consume` | mesmos atributos acima | `cdc_consume` |

#### Exemplo conceitual de span

```
name=amm.swap
attributes={
  "op":"swap",
  "amm.k_before":1.0,
  "amm.k_after":1.05,
  "amm.delta_k_ratio":0.05,
  "amm.fee_ppm":300,
  "amm.input":100.0,
  "amm.output":99.7
}
```

## 3. Log JSON estruturado

Logs são objetos JSON com campos obrigatórios e opcionais descritos abaixo. Campos obrigatórios não podem faltar; campos opcionais devem seguir os formatos especificados.

| Campo | Obrigatório | Tipo/Formato | Observações |
| --- | --- | --- | --- |
| `ts` | Sim | `string` (`date-time` RFC3339 UTC, termina com `Z`) | Ex.: `2025-10-03T12:34:56Z`. |
| `level` | Sim | `string` enum (`trace`, `debug`, `info`, `warn`, `error`) | Sensível a caixa. |
| `msg` | Sim | `string` | Mensagem humana curta. |
| `trace_id` | Sim | `string` (`^[0-9a-f]{32}$`) | Hex minúsculo de 16 bytes. |
| `span_id` | Sim | `string` (`^[0-9a-f]{16}$`) | Hex minúsculo de 8 bytes. |
| `service` | Sim | `string` (`ce-amm`) | Deve coincidir com resource attribute. |
| `env` | Sim | `string` enum (`dev`, `stg`, `prod`) | |
| `op` | Sim | `string` (`swap`, `add_liquidity`, `remove_liquidity`, `pricing`, `cdc_consume`) | Regex `^(swap|add_liquidity|remove_liquidity|pricing|cdc_consume)$`. |
| `version` | Sim | `string` (SemVer ou git SHA) | Ex.: `1.2.3` ou `4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f`. |
| `hook_id` | Não | `string` (`^[a-z0-9]+([-_][a-z0-9]+)*$`) | Identificador estável de hook. |
| `error.kind` | Não | `string` | Tipo curto de erro. |
| `error.message` | Não | `string` | Mensagem curta, sem PII. |
| `extra` | Não | `object` | Chaves estáveis; valores podem ser `string`, `number`, `boolean` ou `object`. |

### 3.1 Exemplo válido

```json
{
  "ts": "2025-10-03T12:34:56Z",
  "level": "info",
  "msg": "swap executed",
  "trace_id": "4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f",
  "span_id": "9a3b7c1d2e3f4a5b",
  "service": "ce-amm",
  "env": "dev",
  "op": "swap",
  "version": "1.2.3",
  "hook_id": "risk-check",
  "extra": { "amm": {"k_before": 1.0, "k_after": 1.05, "delta_k_ratio": 0.05, "fee_ppm": 300 } }
}
```

### 3.2 Exemplo inválido (contém PII)

```json
{
  "ts": "2025-10-03T12:34:56Z",
  "level": "info",
  "msg": "swap",
  "trace_id": "4fd0c2...",
  "span_id": "9a3b7c...",
  "service": "ce-amm",
  "env": "dev",
  "op": "swap",
  "version": "1.2.3",
  "email": "cliente@exemplo.com"
}
```

Motivo: o campo `email` viola a política de PII e deve ser removido ou mascarado.

## 4. Resource attributes

| Chave | Valor | Notas |
| --- | --- | --- |
| `service.name` | `ce-amm` | Constante para todos os emissores. |
| `service.version` | SemVer ou git SHA | Deve refletir a versão real em produção. |
| `deployment.environment` | `dev`, `stg`, `prod` | Mesmo valor utilizado nos logs e métricas. |

## 5. Flags e variáveis de ambiente

| Nome | Valores permitidos | Uso |
| --- | --- | --- |
| `OBSERVABILITY_LEVEL` | `off`, `min`, `full` | Controla o volume de telemetria emitida. |
| `PROM_SCRAPE` | `on`, `off` | Habilita a exposição `/metrics` em ambientes Dev/Stage. |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | URL (ex.: `http://127.0.0.1:4317`) | Endpoint OTLP para exportação em produção. |

## 6. Política de versionamento

O contrato segue SemVer:

* **MAJOR**: renomear ou remover métricas/spans/labels; alterar semântica de campos obrigatórios; mudar os buckets do histograma.
* **MINOR**: adicionar spans, labels ou buckets opcionais sem quebrar consumidores.
* **PATCH**: correções textuais ou de documentação sem impacto estrutural.

Consumidores devem verificar `OBS1_CONTRACT_VERSION` antes de aplicar mudanças. Alterações incompatíveis exigem bump de versão e comunicação prévia.

## 7. FAQ curta

**P: Posso adicionar um label novo para depuração?**
R: Não. Labels fora da lista canônica quebram cardinalidade controlada e são rejeitados.

**P: Como lidar com hooks que ainda não existem?**
R: Registre a métrica `hook_executions_total` somente quando o hook estiver implementado. Não use valores artificiais.

**P: Preciso enviar logs em ambientes sem OTLP?**
R: Sim, use JSON estruturado e respeite o schema. O downstream decidirá sobre exportação.

## 8. Glossário

* **AMM**: Automated Market Maker responsável pelas operações `swap` e de liquidez.
* **CDC**: Change Data Capture, origem dos eventos monitorados por `cdc_lag_seconds`.
* **PII**: Personally Identifiable Information, terminantemente proibida nos logs.
* **PPM**: Parts per million, unidade usada para `amm.fee_ppm`.

## 9. Conformidade cruzada

* Todos os campos e nomes aqui descritos estão refletidos nos arquivos `src/telemetry_contract.rs`, `schemas/obs1_log_record.schema.json` e `schemas/obs1_contract.yaml`.
* A exposição em `/metrics` deve utilizar exatamente os nomes de métricas aprovados e apenas os buckets listados.
* Antes de publicar alterações, execute as checagens desta thread para garantir aderência ao contrato.

