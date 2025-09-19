Q1 • Observabilidade — Artefatos


Executar localmente (traces via Jaeger OTLP/HTTP):
docker run --rm -it -p 16686:16686 -p 4318:4318 jaegertracing/all-in-one:latest
export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318/v1/traces
export CE_COMMIT_SHA=$(git rev-parse --short HEAD 2>/dev/null || echo unknown)
export OTEL_SERVICE_NAME=credit-engine-core
cargo run --bin obs_demo


UI do Jaeger: http://localhost:16686 (Service: credit-engine-core)


Comentário Jira (sugestão):
Status: Implementação de observabilidade concluída.
Entregas: histogramas swap_latency_ms (ms) e invariant_error_rel (1); tracing com git_commit_sha; export via OTLP/HTTP (0.29).
Validação: Jaeger recebendo traces em :4318/v1/traces e UI em :16686.
Nota: Jaeger não recebe métricas OTLP. Para métricas, abrir task separada com OTel Collector.
Sem push nesta sprint (local-only), conforme escopo.
