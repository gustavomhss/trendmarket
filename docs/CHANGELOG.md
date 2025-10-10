# CHANGELOG

## [5.0.0] - 2024-05-21
### Added
- OBS-3 Prometheus configs for dev/prod with shared recording rules and label hygiene.
- Recording rules and promtool tests covering healthy and heavy-tail latency profiles.
- Evidence scripts (`obs_t3_prom_scrape_run.sh`, `obs3_quality_checks.py`,
  `obs3_hash_manifest.py`, `obs3_verify_manifest.py`, `obs3_all_checks.sh`).
- JSON schema for manifests plus validated example.
- GitHub Actions workflow (`obs3-prometheus-ci.yml`) and semantic PR guard.
- Governance pack: labels, CODEOWNERS, branch protection template, GH helper script.
- Documentation: executive summary, QA checklist, RFC template, README additions.
- Makefile, pre-commit hooks, and requirements to run OBS-3 toolchain locally.
