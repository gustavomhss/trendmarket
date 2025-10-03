# OBS-1 Contract Consumption Guide

Este guia explica como consumir o contrato OBS-1 definido nesta thread.

## 1. Referências de código

Use o módulo `telemetry_contract` para importar todos os identificadores:

```rust
use trendmarket::telemetry_contract::{
    OBS1_CONTRACT_VERSION,
    METRIC_AMM_OP_LATENCY_SECONDS,
    AMM_OP_LATENCY_BUCKETS,
    LABELS_PERMITIDOS,
    OPERATION_VALUES,
};
```

Garanta que qualquer emissor de métricas utilize exclusivamente os nomes e labels publicados. Compare labels candidatos contra `LABELS_PERMITIDOS` e rejeite qualquer item em `LABELS_PROIBIDOS`.

## 2. Validação de logs

O schema `schemas/obs1_log_record.schema.json` é a referência para validação estática de logs estruturados. Antes de liberar alterações, valide arquivos de exemplo:

```bash
ajv validate -s schemas/obs1_log_record.schema.json -d docs/obs1_log_example.json
```

Em ambientes sem `ajv`, utilize qualquer validador compatível com Draft-07. Campos PII (`email`, `cpf`, `phone`, `address`, `name`, `geo`, `person_*`) devem ser bloqueados automaticamente.

## 3. Métricas e spans

* Histograma `amm_op_latency_seconds`: use `AMM_OP_LATENCY_BUCKETS` e respeite o conjunto de labels `{op, service, env, version}`.
* Counter `hook_executions_total`: somente incremente com `status` em `{success, error}`.
* Gauges `data_freshness_seconds`, `cdc_lag_seconds` e `drift_score`: publicar apenas valores reais; nada de placeholders.
* Spans devem usar os nomes `amm.swap`, `amm.add_liquidity`, `amm.remove_liquidity`, `pricing.quote` e `cdc.consume`, incluindo todos os atributos `amm.*` definidos.

## 4. Flags e resource attributes

* Configure `service.name=ce-amm`, `service.version=<semver|git_sha>` e `deployment.environment` conforme ambiente.
* Flags disponíveis: `OBSERVABILITY_LEVEL` (`off|min|full`), `PROM_SCRAPE` (`on|off`) e `OTEL_EXPORTER_OTLP_ENDPOINT` (URL OTLP).

## 5. Versionamento e breaking changes

* Verifique `OBS1_CONTRACT_VERSION` antes de introduzir novidades.
* Alterações incompatíveis exigem incremento MAJOR e comunicação prévia.
* Adições compatíveis (labels opcionais, novos spans) usam incremento MINOR.
* Correções textuais usam PATCH.

## 6. Checklist rápido

- [ ] Utilize apenas métricas e spans canônicos.
- [ ] Valide todos os logs contra o schema JSON.
- [ ] Rejeite labels fora de `LABELS_PERMITIDOS` e campos PII.
- [ ] Garanta alinhamento de `op` entre métricas, spans e logs.
- [ ] Atualize dependentes ao mudar `OBS1_CONTRACT_VERSION`.

## 7. Perguntas frequentes

**Como lidar com ambientes sem Prometheus?**
Use `OBSERVABILITY_LEVEL=min` e mantenha o histograma local; a exportação OTLP pode ser desativada definindo `PROM_SCRAPE=off`.

**Posso adicionar métricas novas?**
Não nesta thread. Submeta uma proposta com bump de versão seguindo SemVer.

**Como validar o histograma?**
Compare os limites dos buckets com `AMM_OP_LATENCY_BUCKETS` e configure o SDK OTel para usar o modo explícito.

