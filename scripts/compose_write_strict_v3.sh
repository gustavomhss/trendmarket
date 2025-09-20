#!/usr/bin/env bash
set -euo pipefail
[ -f ./.env ] && set -a && . ./.env && set +a
: "${HTTP_PORT:=4318}"; : "${GRPC_PORT:=4317}"; : "${JAEGER_UI:=16686}"; : "${JAEGER_GRPC:=14250}"; : "${PROM_WEB:=9090}"


# escreve com espaços explícitos
{
printf '%s\n' 'services:'
printf '%s\n' ' otel-collector:'
printf '%s\n' ' image: otel/opentelemetry-collector:0.99.0'
printf '%s\n' ' container_name: otel-collector'
printf '%s\n' ' command: ["--config", "/etc/otelcol/config.yaml"]'
printf '%s\n' ' volumes:'
printf '%s\n' ' - ./ops/otel/otelcol:/etc/otelcol:ro'
printf '%s\n' ' ports:'
printf '%s\n' ' - "${GRPC_PORT:-4317}:4317"'
printf '%s\n' ' - "${HTTP_PORT:-4318}:4318"'
printf '%s\n' ' - "13133:13133"'
printf '%s\n' ' depends_on:'
printf '%s\n' ' - jaeger'
printf '%s\n' ''
printf '%s\n' ' jaeger:'
printf '%s\n' ' image: jaegertracing/all-in-one:1.60'
printf '%s\n' ' container_name: jaeger'
printf '%s\n' ' environment:'
printf '%s\n' ' - COLLECTOR_OTLP_ENABLED=true'
printf '%s\n' ' ports:'
printf '%s\n' ' - "${JAEGER_UI:-16686}:16686"'
printf '%s\n' ' - "${JAEGER_GRPC:-14250}:14250"'
printf '%s\n' ''
printf '%s\n' ' prometheus:'
printf '%s\n' ' image: prom/prometheus:v3.0.0'
printf '%s\n' ' container_name: prometheus'
printf '%s\n' ' command: ["--config.file=/etc/prometheus/prometheus.yml", "--web.enable-otlp-receiver"]'
printf '%s\n' ' volumes:'
printf '%s\n' ' - ./ops/otel/prometheus:/etc/prometheus:ro'
printf '%s\n' ' ports:'
printf '%s\n' ' - "${PROM_WEB:-9090}:9090"'
} > docker-compose.observability.yml


# exibe com numeração p/ auditoria
nl -ba docker-compose.observability.yml | sed -n '1,160p'


# validação estrutural
if docker compose -f docker-compose.observability.yml config >/dev/null; then
echo '[compose] ✅ YAML válido'
else
echo '[compose] ❌ YAML inválido'; exit 2
fi
