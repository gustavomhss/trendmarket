#!/usr/bin/env bash
set -euo pipefail
mkdir -p ops/otel/prometheus ops/otel/otelcol

if [ -e ops/otel/prometheus.yml ] && [ -d ops/otel/prometheus.yml ]; then
  echo "[paths] ⚠️ 'ops/otel/prometheus.yml' é diretório — movendo p/ backup"
  mv ops/otel/prometheus.yml ops/otel/prometheus.yml.bak.dir
fi

if [ -f ops/otel/prometheus.yml ]; then
  echo "[paths] ⚠️ movendo 'ops/otel/prometheus.yml' para 'ops/otel/prometheus/prometheus.yml'"
  mv -f ops/otel/prometheus.yml ops/otel/prometheus/prometheus.yml
fi

cat > ops/otel/prometheus/prometheus.yml <<'YAML'
global:
  scrape_interval: 5s

scrape_configs:
  - job_name: "otel-collector"
    static_configs:
      - targets: ["otel-collector:9464"]
YAML

cat > ops/otel/otelcol/config.yaml <<'YAML'
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch: {}

exporters:
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true
  prometheus:
    endpoint: 0.0.0.0:9464
    namespace: credit_engine

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [jaeger]
    metrics:
      receivers: [otlp]
      processors: [batch]
      exporters: [prometheus]
YAML

echo "[paths] ✅ ok: ops/otel/{prometheus/prometheus.yml, otelcol/config.yaml}"
