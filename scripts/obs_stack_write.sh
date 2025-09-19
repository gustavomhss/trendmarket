#!/usr/bin/env bash
set -euo pipefail
if ! command -v docker >/dev/null 2>&1; then echo "[ERRO] docker não encontrado. Instale Docker Desktop para Mac." >&2; exit 1; fi
if ! docker info >/dev/null 2>&1; then echo "[ERRO] Docker daemon não está rodando. Abra o Docker Desktop." >&2; exit 1; fi
mkdir -p ops/otel ops/grafana/dashboards ops/grafana/provisioning/dashboards ops/grafana/provisioning/datasources
cat > ops/otel/otelcol.yaml <<'YML'
receivers:
otlp:
protocols:
http:
grpc:
processors:
batch:
exporters:
logging:
loglevel: info
jaeger:
endpoint: jaeger:14250
tls:
insecure: true
prometheus:
endpoint: "0.0.0.0:9464"
service:
pipelines:
traces:
receivers: [otlp]
processors: [batch]
exporters: [jaeger, logging]
metrics:
receivers: [otlp]
processors: [batch]
exporters: [prometheus, logging]
YML
cat > docker-compose.observability.yml <<'YML'
services:
otel-collector:
image: otel/opentelemetry-collector:0.99.0
container_name: otel-collector
command: ["--config", "/etc/otelcol/config.yaml"]
volumes:
- ./ops/otel/otelcol.yaml:/etc/otelcol/config.yaml:ro
ports:
- "4317:4317"
- "4318:4318"
- "9464:9464"
depends_on: [jaeger]
jaeger:
image: jaegertracing/all-in-one:1.60
container_name: jaeger
environment:
- COLLECTOR_OTLP_ENABLED=true
ports:
- "16686:16686"
- "14250:14250"
prometheus:
image: prom/prometheus:v3.0.0
container_name: prometheus
command:
- --config.file=/etc/prometheus/prometheus.yml
volumes:
- ./ops/otel/prometheus.yml:/etc/prometheus/prometheus.yml:ro
ports:
- "9090:9090"
depends_on: [otel-collector]
grafana:
image: grafana/grafana:10.4.2
container_name: grafana
environment:
- GF_SECURITY_ADMIN_PASSWORD=admin
volumes:
- ./ops/grafana/dashboards:/var/lib/grafana/dashboards
- ./ops/grafana/provisioning:/etc/grafana/provisioning
ports:
- "3000:3000"
depends_on: [prometheus]
YML
cat > ops/otel/prometheus.yml <<'YML'
global: { scrape_interval: 5s }
scrape_configs:
- job_name: 'otel-collector'
static_configs: [ { targets: ['otel-collector:9464'] } ]
YML
cat > ops/grafana/provisioning/dashboards/dashboards.yml <<'YML'
apiVersion: 1
providers:
- name: 'CE Dashboards'
orgId: 1
folder: ''
type: file
disableDeletion: false
editable: true
options: { path: /var/lib/grafana/dashboards }
YML
cat > ops/grafana/provisioning/datasources/prometheus.yml <<'YML'
apiVersion: 1
datasources:
- name: Prometheus
type: prometheus
access: proxy
url: http://prometheus:9090
isDefault: true
editable: true
YML
cat > ops/grafana/dashboards/ce_engine_basic.json <<'JSON'
{
"title": "CE Engine — Básico",
"panels": [
{ "type": "timeseries", "title": "swap_latency_ms p95", "targets": [{ "expr": "histogram_quantile(0.95, sum by (le) (rate(swap_latency_ms_bucket[5m])))" }] },
{ "type": "timeseries", "title": "invariant_error_rel (amostrado)", "targets": [{ "expr": "avg(rate(invariant_error_rel_sum[5m]) / rate(invariant_error_rel_count[5m]))" }] }
],
"time": { "from": "now-1h", "to": "now" },
"schemaVersion": 39
}
JSON


echo "[OK] Stack de observabilidade escrita."
