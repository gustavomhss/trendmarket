#!/usr/bin/env bash
set -euo pipefail
mkdir -p ops/otel/otelcol
cat > ops/otel/otelcol/config.yaml <<'YAML'
extensions:
health_check:
endpoint: 0.0.0.0:13133


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
extensions: [health_check]
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


echo "[healthz] ✅ config.yaml com /healthz habilitado (13133)"
