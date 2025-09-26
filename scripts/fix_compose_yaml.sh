#!/usr/bin/env bash
set -euo pipefail
mkdir -p ops/otel
cat > docker-compose.observability.yml <<'YML'
version: "3.9"
services:
otel-collector:
image: otel/opentelemetry-collector:0.99.0
container_name: otel-collector
command: ["--config", "/etc/otelcol/config.yaml"]
volumes:
- ./ops/otel/otelcol-config.yaml:/etc/otelcol/config.yaml:ro
ports:
- "4317:4317" # gRPC
- "4318:4318" # HTTP
- "9464:9464" # Prometheus exporter
depends_on:
- jaeger


jaeger:
image: jaegertracing/all-in-one:1.60
container_name: jaeger
environment:
- COLLECTOR_OTLP_ENABLED=true
ports:
- "16686:16686" # UI
- "14250:14250" # gRPC ingest


prometheus:
image: prom/prometheus:v3.0.0
container_name: prometheus
command: ["--config.file=/etc/prometheus/prometheus.yml", "--web.enable-otlp-receiver"]
volumes:
- ./ops/otel/prometheus.yml:/etc/prometheus/prometheus.yml:ro
ports:
- "9090:9090"
YML


cat > ops/otel/otelcol-config.yaml <<'YAML'
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


cat > ops/otel/prometheus.yml <<'YAML'
global:
scrape_interval: 5s
scrape_configs:
- job_name: "otel-collector"
static_configs:
- targets: ["otel-collector:9464"]
YAML


# Valida YAML
if ! docker compose -f docker-compose.observability.yml config > /dev/null; then
echo "[compose] ❌ YAML inválido"; exit 2
fi


echo "[compose] ✅ YAML válido e container_name fixado (otel-collector/jaeger/prometheus)."
