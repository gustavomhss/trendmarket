# OBS‑1 — Resumo de diagnóstico (pós ajustes manuais)

## A) Totais por arquivo de log
- check-after-manual.txt: 0
0
- test-norun-after-manual.txt: 0
0
- test-run-after-manual.txt: 0
0

## B) Agrupado por código de erro (E0xxx)

## C) Top 10 arquivos com mais erros

## D) Assinaturas OTel/Prometheus potencialmente incompatíveis

### scan: with-tonic.txt

### scan: periodicreader-builder.txt

### scan: with-timeout.txt

### scan: prom-exporter-apis.txt
src/telemetry_metrics_prom.rs:33:        self.exporter.provider()
tests/telemetry_metrics_prom_tests.rs:14:    let provider = exporter.meter_provider();
tests/telemetry_metrics_prom_tests.rs:35:    let guard_provider = guard_exporter.meter_provider();

### scan: prometheus-compat.txt
src/telemetry.rs:7:use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

## E) Árvores de dependência relevantes

### opentelemetry_sdk

### opentelemetry-otlp

### prometheus

### metrics-exporter-prometheus
