#!/usr/bin/env bash
set -euo pipefail
[ -f ./.env ] && set -a && . ./.env && set +a
: "${HTTP_PORT:=4318}"; : "${GRPC_PORT:=4317}"; : "${JAEGER_UI:=16686}"; : "${JAEGER_GRPC:=14250}"; : "${PROM_WEB:=9090}"


cat > docker-compose.observability.yml <<'YML'
services:
otel-collector:
image: otel/opentelemetry-collector:0.99.0
container_name: otel-collector
command: ["--config", "/etc/otelcol/config.yaml"]
volumes:
- ./ops/otel/otelcol:/etc/otelcol:ro
ports:
- "${GRPC_PORT:-4317}:4317"
- "${HTTP_PORT:-4318}:4318"
- "13133:13133"
depends_on:
- jaeger


jaeger:
image: jaegertracing/all-in-one:1.60
container_name: jaeger
environment:
- COLLECTOR_OTLP_ENABLED=true
ports:
- "${JAEGER_UI:-16686}:16686"
- "${JAEGER_GRPC:-14250}:14250"


prometheus:
image: prom/prometheus:v3.0.0
container_name: prometheus
command: ["--config.file=/etc/prometheus/prometheus.yml", "--web.enable-otlp-receiver"]
volumes:
- ./ops/otel/prometheus:/etc/prometheus:ro
ports:
- "${PROM_WEB:-9090}:9090"
YML


# auditoria visual (confira os espaços no início das linhas!)
nl -ba docker-compose.observability.yml | sed -n '1,200p'


# validação sintática clara
docker compose -f docker-compose.observability.yml config >/dev/null \
&& echo "[compose] ✅ YAML válido" \
|| { echo "[compose] ❌ YAML inválido"; exit 2; }
