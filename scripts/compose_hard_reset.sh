#!/usr/bin/env bash
set -euo pipefail

rm -f docker-compose.observability.yml
rm -rf ops/otel
mkdir -p ops/otel

is_free() {
  local p=$1
  if command -v lsof >/dev/null 2>&1; then
    ! lsof -i ":$p" -sTCP:LISTEN -Pn >/dev/null 2>&1
  elif command -v nc >/dev/null 2>&1; then
    ! nc -z localhost "$p" >/dev/null 2>&1
  else
    # bash /dev/tcp fallback
    (echo > /dev/tcp/127.0.0.1/$p) >/dev/null 2>&1 && return 1 || return 0
  fi
}

pick_port() {
  local want=$1 alt=$2
  if is_free "$want"; then echo "$want"; else echo "$alt"; fi
}

HTTP_PORT=$(pick_port 4318 4320)
GRPC_PORT=$(pick_port 4317 4319)
PROM_PORT=$(pick_port 9464 9465)
JAEGER_UI=$(pick_port 16686 16696)
JAEGER_GRPC=$(pick_port 14250 14260)
PROM_WEB=$(pick_port 9090 9099)

# Compose usa .env automaticamente
cat > .env <<ENV
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:${HTTP_PORT}
JAEGER_URL=http://localhost:${JAEGER_UI}
PROM_URL=http://localhost:${PROM_WEB}
HTTP_PORT=${HTTP_PORT}
GRPC_PORT=${GRPC_PORT}
PROM_PORT=${PROM_PORT}
JAEGER_UI=${JAEGER_UI}
JAEGER_GRPC=${JAEGER_GRPC}
PROM_WEB=${PROM_WEB}
ENV

# Compose limpo
cat > docker-compose.observability.yml <<YML
services:
  otel-collector:
    image: otel/opentelemetry-collector:0.99.0
    container_name: otel-collector
    command: ["--config", "/etc/otelcol/config.yaml"]
    volumes:
      - ./ops/otel/otelcol-config.yaml:/etc/otelcol/config.yaml:ro
    ports:
      - "\${GRPC_PORT}:4317"
      - "\${HTTP_PORT}:\${HTTP_PORT}"
    depends_on:
      - jaeger

  jaeger:
    image: jaegertracing/all-in-one:1.60
    container_name: jaeger
    environment:
      - COLLECTOR_OTLP_ENABLED=true
    ports:
      - "\${JAEGER_UI}:16686"
      - "\${JAEGER_GRPC}:14250"

  prometheus:
    image: prom/prometheus:v3.0.0
    container_name: prometheus
    command: ["--config.file=/etc/prometheus/prometheus.yml", "--web.enable-otlp-receiver"]
    volumes:
      - ./ops/otel/prometheus.yml:/etc/prometheus/prometheus.yml:ro
    ports:
      - "\${PROM_WEB}:9090"
YML

# Config do Collector
cat > ops/otel/otelcol-config.yaml <<YAML
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:${HTTP_PORT}

processors:
  batch: {}

exporters:
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true
  prometheus:
    endpoint: 0.0.0.0:${PROM_PORT}

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

chmod +x scripts/compose_hard_reset.sh
