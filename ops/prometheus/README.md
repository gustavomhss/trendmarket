# OBS-3 Prometheus Packs

This package delivers the OBS-3 / CRD-8 monitoring baseline for the CreditEngine (CE) stack. It ships production-ready scrape
configs, recording rules, test fixtures, and governance hooks that guarantee parity between development and production
clusters.

## Architecture overview

```text
┌─────────────────────┐      ┌────────────────────┐      ┌─────────────────────┐
│ CE Core Pods        │ ---> │ Prometheus Agent   │ ---> │ Central Prometheus  │
│ (histograms/hooks)  │      │ (dev/prod scrape)  │      │ (rules & alerts)    │
└─────────────────────┘      └────────────────────┘      └─────────────────────┘
        │                              │                           │
        ▼                              │                           ▼
┌─────────────────────┐                │                ┌──────────────────────┐
│ OpenTelemetry       │  <-------------┘                │ Evidence Manifests   │
│ Collector (otelcol) │                                 │ & OBS-3 Gatechecks   │
└─────────────────────┘                                 └──────────────────────┘
```

## Dev ↔ Prod parity

| Aspect              | Dev                                          | Prod                                                |
| ------------------- | -------------------------------------------- | --------------------------------------------------- |
| Scrape discovery    | Static `localhost` targets                   | File SD with RFC1918 targets (`targets-*.json`)     |
| External labels     | `{ env: dev, stack: ce }`                    | `{ env: prod, stack: ce }`                          |
| Rule files          | `rules/core.rules.yml` (shared)              | `rules/core.rules.yml` (shared)                     |
| Metric hygiene      | Drop `instance|pod|container|namespace|endpoint` in both environments |
| Alert automation    | OBS-3 scripts emit manifest & hashes for Gate OBS-3 | Same scripts executed via CI + production runbooks |

## Catalog of recordings

| Recording                                   | Description                                                      | Labels                           |
| ------------------------------------------ | ---------------------------------------------------------------- | -------------------------------- |
| `ce:amm_op_latency_seconds:p75`            | Latency p75 over a 5m window for each AMM operation              | `agg`, `op`, `service`           |
| `ce:amm_op_latency_seconds:p95`            | Latency p95 over a 5m window                                     | `agg`, `op`, `service`           |
| `ce:amm_op_latency_seconds:avg`            | Rolling 5m mean derived from `_sum` / `_count`                   | `agg`, `op`, `service`           |
| `ce:amm_hook_invocations:throughput_5m`    | Hook throughput rate (5m) for incident automation hooks          | `agg`, `hook`, `service`         |
| `ce:data_feature:max_5m`                   | Max feature value observed per source/stream/feature in 5m       | `agg`, `service`, `source`, `stream`, `feature` |

### Anti-patterns avoided

- **No le label drops** — only infrastructure-specific labels are removed.
- **No sparse histograms** — bucket coverage includes realistic service latency ranges (0.1s → 1s).
- **No counter resets** — test suites enforce monotonic `_bucket`, `_sum`, `_count`, and hook counters.

## Useful queries

- `ce:amm_op_latency_seconds:p95{service="ce-core"}` — tail latency guardrail for Gate A110.
- `ce:amm_hook_invocations:throughput_5m{hook="defer-settlement"}` — throughput of automation hooks.
- `ce:data_feature:max_5m{stream="loan_decisions"}` — most recent data spikes feeding pricing models.

## Testing locally

```bash
promtool check config ops/prometheus/prometheus.dev.yml
promtool test rules ops/prometheus/tests/core.rules.test.yml
./scripts/obs3_all_checks.sh
```

## Evidence workflow

1. Run `./scripts/obs_t3_prom_scrape_run.sh --env dev` (or `--env prod` pointing to the remote Prometheus API).
2. The runner stores telemetry under `out/obs_gatecheck/prometheus/<run_id>/`.
3. `obs3_quality_checks.py` validates histogram monotonicity, quantile ordering, counter behaviour, and exports
   `quality_report.json`.
4. `obs3_hash_manifest.py` generates `manifest.json` + SHA256 digests for the evidence.
5. `obs3_verify_manifest.py` validates the manifest against `ops/schemas/manifest.schema.json`.

All outputs are safe for auditors and are consumed by the OBS-3 CI workflow.
