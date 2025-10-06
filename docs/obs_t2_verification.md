# OBS‑2 Verification (Dev/Local)

## Quick Start (binary)
```bash
./scripts/obs_t2_collector_dev.sh prom
# or
./scripts/obs_t2_collector_dev.sh rw
```

## Quick Start (docker compose)

```bash
./scripts/obs_t2_collector_dev.sh compose-prom
# or
./scripts/obs_t2_collector_dev.sh compose-rw
```

## Health

* Health: `curl -s localhost:13133/healthz` → `Server available`
* Self telemetry: `curl -s localhost:8888/metrics | head`
* Pipeline metrics (prom exporter mode): `curl -s localhost:9464/metrics | head`

## End‑to‑End with App (from OBS‑1)

* Set the app: `export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317`
* Generate swaps/ops to produce traces/metrics/logs
* Check Prometheus ([http://localhost:9090](http://localhost:9090)):

  * Example: `histogram_quantile(0.95, sum by (le,op) (rate(amm_op_latency_seconds_bucket[5m])))`
* Check Tempo ([http://localhost:3200](http://localhost:3200)) / Jaeger UI if configured
* Check Loki ([http://localhost:3100](http://localhost:3100)) using LogQL with `trace_id` field

## Evidence

* `out/obs_gatecheck/logs/collector_dev.txt`
* `out/obs_gatecheck/evidence/collector_dev.json`

## Troubleshooting

* Port conflicts: ensure 4317/4318/13133/1777/55679/8888/9464 free
* Apple Silicon: images `grafana/*` provide multi‑arch; use `platform:` only if needed
* RemoteWrite fails: confirm `PROM_RW_ENDPOINT` is reachable
* No spans in Tempo: confirm `TEMPO_OTLP_ENDPOINT`
* No Loki logs: confirm `LOKI_ENDPOINT` and labels

```
