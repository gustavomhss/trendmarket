# CE — Q1 Observability (Artefatos)
- compose: docker-compose.observability.yml
- otelcol: ops/otel/otelcol/config.yaml
- prometheus: ops/prometheus/prometheus.yml
- grafana dashboard: ops/grafana/dashboards/ce_engine_basic.json


Aceite atendido:
- Métricas: swap_latency_ms, invariant_error_rel
- Tracing: trace_id + commit_sha (exposto como git.commit.sha)
- Dashboard básico no Grafana
