# Manifest Schema & Verifier (Thread 7)

This thread delivers the JSON Schema and Python verifier that enforce the OBS-3
manifest contract. Use them to guarantee every evidence package is structurally
sound before it feeds the downstream packs (threads 4-6).

## Required fields (summary)

The canonical schema is published at
[`ops/schemas/manifest.schema.json`](../ops/schemas/manifest.schema.json). It
requires the following top-level keys:

- `run_id` (UUID v4)
- `spec_version` (must be `"5.0"`)
- `git_sha`
- `ts` (ISO 8601 `date-time`)
- `targets`, `rules`, `up`
- `p75`, `p95`
- `series_sample`
- `cardinality_snapshot`
- `quality_checks`
- `integrity`

Optional context lives in `notes` (≤2000 chars). Each checksum in `integrity`
must be a lowercase SHA-256 hex digest.

## Usage

Install the `jsonschema` package in your Python environment first `pip install jsonschema`.

```bash
python3 scripts/obs3_verify_manifest.py \
  --manifest out/obs_gatecheck/evidence/prom_scrape.json \
  --schema ops/schemas/manifest.schema.json
```

Pass `--pretty` for CLI compatibility (no visual change) and `--strict` to plan
for future hardening; both flags are currently no-ops.

## Exit codes

| Code | Meaning |
| ---- | ------- |
| 0    | Manifest is valid |
| 4    | Schema missing or unreadable |
| 5    | Manifest missing or unreadable |
| 6    | Invalid JSON payload (schema or manifest) |
| 7    | Validation errors detected |
| 9    | Unexpected internal error |

## Troubleshooting

Common rejection reasons:

- `spec_version` not pinned to `"5.0"`
- `integrity` empty or containing non-SHA-256 values
- Missing `quality_checks` booleans or nested telemetry fields
- Arrays (`p75`, `p95`) missing `metric`, `value`, or `labels`
- `cardinality_snapshot` counters below zero

Validate locally with the provided example manifest in
[`ops/schemas/examples/prom_scrape.example.json`](../ops/schemas/examples/prom_scrape.example.json)
before shipping new evidence.
