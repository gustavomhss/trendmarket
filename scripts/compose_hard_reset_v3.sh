#!/usr/bin/env bash
set -euo pipefail


rm -f docker-compose.observability.yml


is_free(){ local p=$1; if command -v lsof >/dev/null 2>&1; then ! lsof -i ":$p" -sTCP:LISTEN -Pn >/dev/null 2>&1; else ! nc -z localhost "$p" >/dev/null 2>&1; fi }
pick(){ local want=$1 alt=$2; is_free "$want" && echo "$want" || echo "$alt"; }


HTTP_PORT=$(pick 4318 4320)
GRPC_PORT=$(pick 4317 4319)
PROM_PORT=$(pick 9464 9465)
JAEGER_UI=$(pick 16686 16696)
JAEGER_GRPC=$(pick 14250 14260)
PROM_WEB=$(pick 9090 9099)


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


cat > docker-compose.observability.yml <<YML
services:
otel-collector:
image: otel/opentelemetry-collector:0.99.0
container_name: otel-collector
command: ["--config", "/etc/otelcol/config.yaml"]
volumes:
- ./ops/otel/otelcol:/etc/otelcol:ro
ports:
- "${GRPC_PORT}:4317"
- "${HTTP_PORT}:4318"
depends_on:
- jaeger


jaeger:
image: jaegertracing/all-in-one:1.60
container_name: jaeger
environment:
- COLLECTOR_OTLP_ENABLED=true
ports:
- "${JAEGER_UI}:16686"
- "${JAEGER_GRPC}:14250"


prometheus:
image: prom/prometheus:v3.0.0
container_name: prometheus
command: ["--config.file=/etc/prometheus/prometheus.yml", "--web.enable-otlp-receiver"]
volumes:
- ./ops/otel/prometheus:/etc/prometheus:ro
ports:
- "${PROM_WEB}:9090"
YML


# valida YAML
docker compose -f docker-compose.observability.yml config >/dev/null && echo "[compose] ✅ YAML válido"
