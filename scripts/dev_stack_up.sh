#!/usr/bin/env bash
set -euo pipefail
export COMPOSE_FILE=docker-compose.observability.yml


[ -f ops/otel/prometheus/prometheus.yml ] || { echo "[stack] ❌ falta ops/otel/prometheus/prometheus.yml"; exit 2; }
[ -f ops/otel/otelcol/config.yaml ] || { echo "[stack] ❌ falta ops/otel/otelcol/config.yaml"; exit 2; }


# valida estruturalmente o compose antes de subir
if ! docker compose -f "$COMPOSE_FILE" config >/dev/null; then
echo "[stack] ❌ docker-compose inválido"; exit 2
fi


docker compose down -v --remove-orphans >/dev/null 2>&1 || true


echo "[stack] pull de imagens (pode demorar na 1ª vez)…"; docker compose pull


echo "[stack] subindo serviços…"
if ! docker compose up -d --force-recreate; then
echo "[stack] ❌ falha ao subir — logs recentes:";
docker logs prometheus --tail=120 2>/dev/null || true
docker logs otel-collector --tail=120 2>/dev/null || true
docker logs jaeger --tail=120 2>/dev/null || true
exit 2
fi


echo "\n[stack] containers:" && docker ps --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'


[ -f ./.env ] && set -a && . ./.env && set +a


echo "\nJaeger: ${JAEGER_URL:-http://localhost:16686}"
echo "Prometheus: ${PROM_URL:-http://localhost:9090}"
echo "OTLP HTTP: ${OTEL_EXPORTER_OTLP_ENDPOINT:-http://localhost:4318}"
