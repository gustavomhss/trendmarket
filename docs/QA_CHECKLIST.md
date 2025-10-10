# OBS-3 QA Checklist (RAG)

| Item | Owner | Status | Evidence |
| ---- | ----- | ------ | -------- |
| Prometheus config validated via `promtool check config` (dev/prod) | SRE | ⬜️ | logs/promtool_check.log |
| Recording rules tested (`promtool test rules`) | SRE | ⬜️ | logs/promtool_rules_test.log |
| OBS-3 quality checks executed (`obs3_quality_checks.py`) | Observability | ⬜️ | quality_report.json |
| Manifest hashed & verified (`obs3_hash_manifest.py` / `obs3_verify_manifest.py`) | Compliance | ⬜️ | manifest.json |
| CI workflow `obs3-prometheus-ci.yml` green | Platform | ⬜️ | GitHub Actions |
| Governance artifacts applied (`gh_setup_repo_policies.sh`) | Admin | ⬜️ | GitHub Settings |
| Documentation refreshed (`README`, `EXEC_SUMMARY`, runbooks) | Docs | ⬜️ | docs/ |
