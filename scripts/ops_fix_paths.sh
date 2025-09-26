#!/usr/bin/env bash
set -euo pipefail
mkdir -p ops/otel


fix_file() {
local p="$1"
local content="$2"
if [ -e "$p" ] && [ -d "$p" ]; then
echo "[paths] ⚠️ '$p' é um diretório — movendo para '$p.bak.dir'"
mv "$p" "$p.bak.dir"
fi
# garante arquivo com conteúdo
printf "%s" "$content" > "$p"
}


PROM_CONTENT='global:\n scrape_interval: 5s\nscrape_configs:\n - job_name: "otel-collector"\n static_configs:\n - targets: ["otel-collector:${PROM_PORT:-9464}"]\n'


# Nota: ${PROM_PORT} será interpolado se .env já existir; senão cai no 9464.


OTELCOL_CONTENT='receivers:\n otlp:\n protocols:\n grpc:\n endpoint: 0.0.0.0:4317\n http:\n endpoint: 0.0.0.0:'"${HTTP_PORT:-4318}"'\nprocessors:\n batch: {}\nexporters:\n jaeger:\n endpoint: jaeger:14250\n tls:\n insecure: true\n prometheus:\n endpoint: 0.0.0.0:'"${PROM_PORT:-9464}"'\n namespace: credit_engine\nservice:\n pipelines:\n traces:\n receivers: [otlp]\n processors: [batch]\n exporters: [jaeger]\n metrics:\n receivers: [otlp]\n processors: [batch]\n exporters: [prometheus]\n'


fix_file ops/otel/prometheus.yml "$PROM_CONTENT"
fix_file ops/otel/otelcol-config.yaml "$OTELCOL_CONTENT"


echo "[paths] ✅ arquivos OK em ops/otel/{prometheus.yml,otelcol-config.yaml}"
