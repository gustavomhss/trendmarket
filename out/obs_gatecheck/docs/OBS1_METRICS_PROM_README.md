# OBS-1 — Prometheus `/metrics` Quickstart

## 1. Preparar ambiente
1. Compile dependências (`cargo build` opcional — o módulo não requer binário adicional).
2. Configure flags de scrape:
   ```bash
   export PROM_SCRAPE=on
   export METRICS_HTTP_ADDR=${METRICS_HTTP_ADDR:-0.0.0.0:9464}
   ```
3. Instancie o exporter no processo de aplicação:
   ```rust
   let exporter = telemetry_metrics_prom::init_prom_exporter();
   let provider = exporter.meter_provider();
   let guard = telemetry_metrics_prom::spawn_metrics_http(
       telemetry_metrics_prom::PromServerConfig {
           addr: std::env::var("METRICS_HTTP_ADDR").unwrap_or_else(|_| "0.0.0.0:9464".into()),
       },
       exporter,
   ).await?;
   ```
   Mantenha o `PromServerGuard` vivo enquanto o endpoint estiver habilitado.

## 2. Validar localmente
- Faça um scrape manual:
  ```bash
  curl -sS http://127.0.0.1:9464/metrics | head -n 30
  ```
- Rode os testes automatizados (cobrem subida HTTP, content-type e séries histogram/counter):
  ```bash
  cargo test telemetry_metrics_prom
  ```

## 3. Integração com métricas OTel
- Registre instrumentos via `provider.meter("<scope>")` antes de publicar.
- O pipeline Prometheus é independente do exporter OTLP, permitindo dual-export.
- Falhas de bind/serve retornam `PromHttpError::{Bind,Serve}` para logging/report.

## 4. Operação
- Sem autenticação/TLS — proteger via rede/sidecar quando exposto fora de DEV/STG.
- Encerramento: chame `guard.shutdown().await` (ou deixe o guard sair de escopo para shutdown automático).
- Monitorar logs para eventos `Serve(...)` (I/O) e tratar colisão de porta (`Bind(...)`).
