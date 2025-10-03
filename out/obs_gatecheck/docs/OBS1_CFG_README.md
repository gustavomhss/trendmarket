# OBS-1 Config – Operação Rápida

## TL;DR
- Carregue a configuração chamando `TelemetryConfig::from_env()` durante o bootstrap.
- Builder programático (`TelemetryConfig::builder()`) vence variáveis de ambiente.
- Defaults seguros: service=`ce-amm`, env=`dev`, level=`min`, log=`info`, OTLP desativado.

## Como ligar/desligar

| Ação                         | Passos |
|------------------------------|--------|
| Habilitar `/metrics` local   | `export PROM_SCRAPE=on` e (opcional) `METRICS_HTTP_ADDR=127.0.0.1:9464`. |
| Apontar OTLP para collector  | `export OTEL_EXPORTER_OTLP_ENDPOINT=https://otel.<env>:4317`. Remover barra final. |
| Forçar observabilidade total | `export OBSERVABILITY_LEVEL=full`. |
| Desligar observabilidade     | `export OBSERVABILITY_LEVEL=off` (tracer/meter/logs **não** devem inicializar). |

## Boas práticas
- Sempre defina `SERVICE_VERSION` em staging/prod (sem string vazia).
- Em produção, configure `OTEL_EXPORTER_OTLP_ENDPOINT`; ausência deve disparar erro em runtime (fora desta thread).
- `DENY_DYNAMIC_LABELS` deve permanecer `true` para evitar cardinalidade explosiva.

## Troubleshooting
| Sintoma | Possível causa | Correção |
|---------|----------------|----------|
| `invalid value for environment variable DEPLOY_ENV` | Valor fora de `dev|stg|prod`. | Ajuste a env (`export DEPLOY_ENV=stg`). |
| `expected one of on, off, true, false, 1, 0` | Flag booleana com valor inválido. | Use `on/off` ou `true/false`. |
| Endpoint OTLP sem normalizar | Valor com barra final. | Remova `/` terminal ou confie no loader (que remove automaticamente). |
| Logs continuam verbosos | `LOG_LEVEL` diferente do esperado. | Confirme `export LOG_LEVEL=warn` (aceita case-insensitive). |

## Exemplos rápidos
```bash
# Dev full com métricas locais
export DEPLOY_ENV=dev
export OBSERVABILITY_LEVEL=full
export PROM_SCRAPE=on
export METRICS_HTTP_ADDR=0.0.0.0:9464

# Prod com OTLP
export DEPLOY_ENV=prod
export OBSERVABILITY_LEVEL=full
export SERVICE_NAME=ce-amm
export SERVICE_VERSION=2.1.0
export OTEL_EXPORTER_OTLP_ENDPOINT=https://otel-collector.prod:4317
export LOG_LEVEL=warn
```

## Validação
- `cargo test telemetry_cfg_tests` — cobre casos válidos e inválidos.
- Grep proibido: executar `rg` e garantir ausência de tokens banidos no módulo.
- Evidência disponível em `out/obs_gatecheck/evidence/obs1_cfg_report.json`.
