# OBS-1 • Telemetry Configuration Contract (`telemetry_cfg`)

## 1. Propósito
Este documento define o contrato **imutável** do módulo `telemetry_cfg`. Ele centraliza a leitura, validação e congelamento das
opções de observabilidade da aplicação CreditEngine (CRD-8 / OBS-1). Nenhum side-effect é disparado aqui: o módulo apenas parseia
valores provenientes de **builder programático**, variáveis de ambiente ou defaults canônicos, entregando uma `TelemetryConfig`
pronta para consumo pelos demais módulos (T3–T7).

## 2. Precedência de fontes
1. **Builder** (`TelemetryConfig::builder()`) — sempre vence.
2. **Variáveis de ambiente** — usadas quando o builder não define o campo.
3. **Defaults** — aplicados apenas na ausência das camadas acima.

A tabela abaixo resume os defaults.

| Campo                    | Default               | Observações |
|--------------------------|-----------------------|-------------|
| `service_name`           | `"ce-amm"`           | Regex `^[a-z0-9._-]{3,64}$` |
| `service_version`        | `"0.0.0-dev"`        | String não vazia, ≤ 64 chars |
| `deploy_env`             | `DeployEnv::Dev`      | `dev`, `stg` ou `prod` |
| `level`                  | `ObsLevel::Min`       | `off`, `min`, `full` |
| `prom_scrape`            | `false`               | Somente T6 abre `/metrics` |
| `metrics_http_addr`      | `"0.0.0.0:9464"`     | Host:port válido |
| `otlp_endpoint`          | `None`                | URL `http(s)://host:port`, sem barra final |
| `log_level`              | `"info"`             | `trace|debug|info|warn|error` |
| `deny_dynamic_labels`    | `true`                | Deve permanecer `true` até T14 reforçar |

## 3. Variáveis de ambiente aceitas

| Env var                          | Campo                  | Valores aceitos / validação |
|----------------------------------|------------------------|-----------------------------|
| `SERVICE_NAME`                   | `service_name`         | Regex `^[a-z0-9._-]{3,64}$` (min 3, max 64) |
| `SERVICE_VERSION`                | `service_version`      | String não vazia, ≤ 64 chars |
| `DEPLOY_ENV`                     | `deploy_env`           | `dev`, `stg`, `prod` (case-insensitive) |
| `OBSERVABILITY_LEVEL`            | `level`                | `off`, `min`, `full` (case-insensitive) |
| `PROM_SCRAPE`                    | `prom_scrape`          | `on|off|true|false|1|0` (case-insensitive) |
| `METRICS_HTTP_ADDR`              | `metrics_http_addr`    | `IPv4:port` ou `hostname:port`; porta 10–65535 |
| `OTEL_EXPORTER_OTLP_ENDPOINT`    | `otlp_endpoint`        | URL `http://host:port` ou `https://host:port`, sem path |
| `LOG_LEVEL`                      | `log_level`            | `trace|debug|info|warn|error` |
| `DENY_DYNAMIC_LABELS`            | `deny_dynamic_labels`  | `on|off|true|false|1|0` |

Regras adicionais:
- Espaços são removidos (`trim`).
- Strings vazias **são inválidas** — o módulo retorna `TelemetryError` com mensagem explícita.
- Valores inválidos não sofrem fallback: o load falha, preservando a rastreabilidade.

## 4. Compatibilidade & regras canônicas

### 4.1 Matriz de compatibilidade (`deploy_env` × `ObsLevel` × `otlp_endpoint`)

| `deploy_env` | `level`        | `otlp_endpoint` obrigatório? | Observações |
|--------------|----------------|------------------------------|-------------|
| `Dev`        | `Off`          | Não                          | Traces/métricas/logs devem permanecer desligados. |
| `Dev`        | `Min`/`Full`   | Não                          | Opcional para testes locais. |
| `Stg`        | `Off`          | Não                          | Cenário temporário apenas (não recomendado). |
| `Stg`        | `Min`/`Full`   | Recomendado                  | Exporter OTLP apontando para collector de Stg. |
| `Prod`       | Qualquer nível | **Sim**                      | Ausência deve gerar erro no runtime (fora desta thread). |

### 4.2 Diretrizes que outras threads devem respeitar
- `ObsLevel::Off` → não inicializar tracer, meter ou camada de logs.
- `ObsLevel::Min` → tracer + métricas essenciais com amostragem mínima.
- `ObsLevel::Full` → ativar tracer, métricas e logs completos.
- `prom_scrape=true` → apenas o módulo de métricas (T6) deve abrir `/metrics` no `metrics_http_addr`.
- `deny_dynamic_labels=true` → manter até enforcement na T14, evitando cardinalidade acidental.

## 5. Exemplos normativos

### 5.1 DEV (válido)
```bash
export DEPLOY_ENV=dev
export OBSERVABILITY_LEVEL=full
export PROM_SCRAPE=on
export METRICS_HTTP_ADDR=0.0.0.0:9464
export LOG_LEVEL=info
```

### 5.2 STG com OTLP (válido)
```bash
export DEPLOY_ENV=stg
export OBSERVABILITY_LEVEL=min
export OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector.stg.svc:4317
export PROM_SCRAPE=off
```

### 5.3 PROD (válido)
```bash
export DEPLOY_ENV=prod
export OBSERVABILITY_LEVEL=full
export SERVICE_NAME=ce-amm
export SERVICE_VERSION=2.1.0
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otel-collector.prod:4317
export LOG_LEVEL=warn
```

### 5.4 Casos inválidos (devem falhar)
- `DEPLOY_ENV=production` → mensagem: `invalid value for environment variable DEPLOY_ENV: expected values: dev|stg|prod; received 'production'`.
- `OBSERVABILITY_LEVEL=maximum` → mensagem: `invalid value for environment variable OBSERVABILITY_LEVEL: expected values: off|min|full; received 'maximum'`.
- `PROM_SCRAPE=maybe` → mensagem: `invalid value for environment variable PROM_SCRAPE: expected one of on, off, true, false, 1, 0; received 'maybe'`.
- `OTEL_EXPORTER_OTLP_ENDPOINT=grpc://host:4317` → mensagem: `invalid value for environment variable OTEL_EXPORTER_OTLP_ENDPOINT: URL starting with http:// or https:// followed by host:port; received 'grpc://host:4317'`.

## 6. API para consumidores

```rust
use credit_engine_core::telemetry_cfg::TelemetryConfig;

fn load_cfg() -> TelemetryConfig {
    TelemetryConfig::from_env().expect("configuração OBS-1 válida")
}
```

### Builder programático

```rust
use credit_engine_core::telemetry_cfg::{TelemetryConfig, DeployEnv, ObsLevel};

let cfg = TelemetryConfig::builder()
    .with_service_name("ce-amm")
    .with_deploy_env(DeployEnv::Stg)
    .with_level(ObsLevel::Full)
    .with_prom_scrape(false)
    .with_otlp_endpoint(Some("https://otel.stg.svc:4317".into()))
    .build()?;
```

### Como outros módulos utilizam
1. Chamar `TelemetryConfig::from_env()` no bootstrap (sem efeitos colaterais).
2. Usar `cfg.level` para decidir se tracer/meter/logs serão registrados.
3. `cfg.prom_scrape` + `cfg.metrics_http_addr` guiam apenas o módulo de métricas.
4. `cfg.otlp_endpoint` alimenta o builder do exporter OTLP (quando presente).
5. `cfg.deny_dynamic_labels` deve ser respeitado por registradores de métricas (futuros trabalhos T14).

## 7. FAQ

**Q:** Como bloquear valores dinâmicos vindos de integrações?
**A:** Habilite `deny_dynamic_labels=true` (default). Outros módulos devem rejeitar labels livres quando este flag estiver ativo.

**Q:** O que acontece se o runtime Prod subir sem OTLP?
**A:** O contrato exige erro na inicialização do runtime (implementado fora desta thread). Aqui apenas validamos e documentamos o requisito.

**Q:** Consigo forçar `prom_scrape=true` em Prod?
**A:** Sim, mas deve ser usado apenas para debug controlado. Monitoramento final ocorre via OTLP.

**Q:** Posso sobrescrever `service_version` via builder e ignorar o env?
**A:** Sim. O builder tem precedência total sobre variáveis de ambiente.

## 8. Checklist de conformidade
- [x] Precedência builder > env > default.
- [x] Regex e validações estritas para cada campo.
- [x] Mensagens de erro acionáveis e rastreáveis.
- [x] Campos públicos, imutáveis, sem side-effects.
- [x] Documentação alinhada aos requisitos da OBS-1.
