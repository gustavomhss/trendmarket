# OBS-3 / CRD-8 Executive Summary

The OBS-3 initiative equips the CreditEngine (CE) platform with production-grade
Prometheus instrumentation and evidence workflows. The bundle introduces
hardened scrape configurations for dev/prod, deterministic recording rules, test
fixtures for tail-latency regimes, and automation scripts that mint auditable
manifests for each run.

## Objectives

1. **Zero-regret observability** — latency and throughput KPIs must be queryable
   within 15s and align with Gate A110 guardrails.
2. **Audit-grade evidence** — every execution yields cryptographically hashed
   manifests referencing telemetry exports and validation logs.
3. **Governance hooks** — GitHub workflows, labels, and branch policies enforce
   semantic PRs, two-review merges, and signed commits.

## Highlights

- Prometheus configs with parity between development and production, sharing the
  same recording rules while isolating discovery mechanisms.
- Recording rules track latency p75/p95, rolling averages, hook throughput, and
  data spikes with test coverage for healthy and heavy-tailed scenarios.
- Scripts (`obs_t3_prom_scrape_run.sh`, `obs3_quality_checks.py`,
  `obs3_hash_manifest.py`, `obs3_verify_manifest.py`) implement the OBS-3
  evidence lifecycle with histogram validation, quantile ordering, and manifest
  verification against `ops/schemas/manifest.schema.json`.
- CI workflow (`obs3-prometheus-ci.yml`) orchestrates promtool checks, linters,
  schema validation, and anti-placeholder scans while publishing artifacts.
- Governance toolkit (`labels.yml`, branch protection template, CODEOWNERS,
  `gh_setup_repo_policies.sh`) ensures manual application of repository guards.

## Next steps

- Integrate OBS-3 scripts into the daily release rehearsal so evidence is
  generated alongside Gate A110.
- Expand watchers to trigger automatic evidence generation during incident
  retrofits.
- Link manifests with the compliance data lake for immutable archival.
