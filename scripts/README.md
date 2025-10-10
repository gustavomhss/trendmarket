## Quality Checks (Thread 5)

The OBS-3 quality checks script (`obs3_quality_checks.py`) validates Prometheus
evidence gathered by Thread 4 or retrieved live from a Prometheus endpoint. It
confirms histogram integrity, quantile ordering, counter monotonicity, and
basic scrape telemetry health before materialising the `prom_scrape.json`
manifest.

### Validations performed

* **Histogram monotonicity** for `amm_op_latency_seconds_bucket` using
  `increase(...)` grouped by `op, service, le`.
* **Bucket/count closure** by comparing
  `sum(rate(amm_op_latency_seconds_bucket))` with
  `rate(amm_op_latency_seconds_count)` (max error 3%).
* **Quantile ordering** (`avg ≤ p75 ≤ p95`) and **tail guard** (`p95 ≤ 4× avg`,
  unless `avg < 1e-9`).
* **Hook counter monotonicity** ensuring `rate(hook_executions_total) ≥ 0` per
  `hook_id,status`.
* **Numeric health** by rejecting any NaN/Inf values.
* **Prometheus scrape telemetry** snapshotting p95 duration, average samples
  post relabel, and average target interval length by job.

### Inputs

The script reads evidence JSON files from Thread 4 (default directory
`out/obs_gatecheck/evidence`). Expected artifacts include queries for `up`,
latency histogram buckets (`increase`, `rate`), quantiles (`p75`, `p95`),
latency averages, hook execution counters, and Prometheus telemetry. The
collector typically exports files such as `prom_up.json`, `prom_p75_rec.json`,
`prom_p95_rec.json`, `prom_p75_adhoc.json`, `prom_p95_adhoc.json`,
`prom_series.json`, and Prometheus telemetry snapshots. When the `--live` flag
is used the script performs HTTP queries against the configured Prometheus
address instead of loading local files.

### Usage

```bash
python3 scripts/obs3_quality_checks.py \
  --evidence-dir out/obs_gatecheck/evidence \
  --addr :9090 --live --window 5m --strict
```

Key flags:

* `--manifest` – override output location (`prom_scrape.json` by default).
* `--timeout` – HTTP read timeout (connect timeout fixed at 3 s).
* `--dry-run` – print JSON to stdout without writing.
* `--verbose` – emit additional progress logs.
* `--strict` – treat any incomplete check or missing telemetry as a failure.

### Outputs

The script updates (or creates) `prom_scrape.json` within the evidence
directory. The manifest contains aggregated booleans for each check, the bucket
closure error percentage, a Prometheus telemetry section, and a
`cardinality_snapshot`. Example excerpt:

```json
{
  "quality_checks": {
    "histogram_monotonic": true,
    "bucket_count_closure_error_pct": {"value": 1.2, "threshold": 3.0},
    "avg_le_p75_le_p95": true,
    "p95_le_4x_avg": true,
    "counters_monotonic": true,
    "no_nan_inf": true,
    "prom_telemetry": {
      "available": true,
      "jobs": {
        "prometheus": {
          "scrape_p95_s": 0.22,
          "samples_post_relabel_avg": 4500.0,
          "interval_length_avg_s": 15.0
        }
      }
    }
  },
  "cardinality_snapshot": {
    "amm_op_latency_seconds_bucket": 96,
    "amm_op_latency_seconds_sum": 8,
    "amm_op_latency_seconds_count": 8,
    "hook_executions_total": 6
  }
}
```

### Exit codes & troubleshooting

| Code | Meaning | Suggested action |
| --- | --- | --- |
| 0 | Success | Checks passed and manifest written. |
| 6 | Required datasets missing | Ensure Thread 4 evidence exists or use `--live`. |
| 8 | Validation failure | Inspect which check failed (`--verbose`). |
| 9 | HTTP or timeout error | Verify Prometheus address, increase `--timeout`. |
| 10 | Malformed JSON | Re-generate evidence or repair corrupted files. |
| 11 | NaN/Inf detected | Investigate source metrics for invalid values. |
| 12 | Telemetry absent (strict) | Provide scrape telemetry data or drop `--strict`. |

### Best practices

* Run after at least 5–10 minutes of steady traffic to capture representative
  rates.
* Keep evidence directories versioned together with Thread 4 outputs.
* Investigate and remediate any NaN/Inf values before re-running.
