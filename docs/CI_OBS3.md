# OBS3 Prometheus CI Workflow

This document summarizes the OBS3 Prometheus CI workflow defined in `.github/workflows/obs3-prometheus-ci.yml`.

## Workflow overview

The workflow runs on every push or pull request that touches Prometheus assets, schemas, shared scripts, the Makefile, or the workflow itself. It contains five jobs:

- **`lint-promtool`** downloads `promtool`, validates the Prometheus configuration files, and (when present) checks the rules bundle. The job uploads the aggregated `promtool` output as an artifact to help with offline debugging.
- **`rules-test`** depends on `lint-promtool` and executes `promtool test rules` against the regression suite. It fails fast when the expected rules test manifest is missing.
- **`static-lint`** installs Python linters (`ruff`, `jsonschema`, `yamllint`) and `shellcheck`, then enforces YAML, Python, and shell hygiene over the Prometheus, scripts, and workflow assets.
- **`schema-validate`** reuses Python tooling to execute the OBS3 manifest verifier against the canonical example and schema files.
- **`anti-scans`** guards the tree against placeholders (TODO/FIXME/etc.) and merge-conflict markers.

All jobs run on `ubuntu-latest`. The workflow sets `PROMTOOL_VERSION=2.54.1` and `PYTHON_VERSION=3.11` via top-level `env` variables.

## Permissions

The workflow declares minimal `GITHUB_TOKEN` permissions:

```yaml
permissions:
  contents: read
  actions: write
  pull-requests: read
```

`actions: write` is required for `actions/upload-artifact@v4`. If repository policy revokes that permission, remove the upload step from `lint-promtool` to avoid 403 errors.

## Timeouts and expected duration

Each job defines a timeout (10–15 minutes). In practice, runs complete in a few minutes: the downloads and package installations dominate (`promtool`, `shellcheck`, and Python tooling). If a job approaches the timeout, investigate stalled network calls or unusually large lint surfaces.

## Troubleshooting failures

| Job | Symptom | Remediation |
| --- | --- | --- |
| `lint-promtool` | Step `Install promtool` fails | Confirm outbound network access; ensure `PROMTOOL_VERSION` matches a published Prometheus release. |
| `lint-promtool` | `Missing ops/prometheus/...` | Add the required configuration files or update paths if they moved. |
| `lint-promtool` | Prometheus lint errors | Run `promtool check config`/`promtool check rules` locally and fix syntax issues. |
| `rules-test` | `Missing ops/prometheus/tests/core.rules.test.yml` | Restore or regenerate the rules test suite. |
| `rules-test` | Rule test failures | Execute `promtool test rules ops/prometheus/tests/core.rules.test.yml` locally to inspect failing scenarios. |
| `static-lint` | `yamllint` violations | Run `yamllint ops/prometheus .github/workflows/obs3-prometheus-ci.yml` and apply suggested fixes (spacing, indentation, ordering). |
| `static-lint` | `shellcheck` diagnostics | Inspect the reported shell scripts, fix quoting/conditionals, and rerun `shellcheck -S warning scripts/*.sh`. |
| `static-lint` | `ruff` errors | Execute `ruff check scripts` to lint Python helpers and resolve style/quality issues. |
| `schema-validate` | Missing files | Ensure the schema (`ops/schemas/manifest.schema.json`), example manifest, and verifier script exist. |
| `schema-validate` | Validation failures | Run `python scripts/obs3_verify_manifest.py --manifest ops/schemas/examples/prom_scrape.example.json --schema ops/schemas/manifest.schema.json` locally and address schema mismatches. |
| `anti-scans` | Placeholder/conﬂict detections | Remove the flagged tokens or conflict markers; the job prints offending paths and line numbers. |

## Running checks locally

To replicate CI locally:

1. Install `promtool ${PROMTOOL_VERSION}` and run:
   ```bash
   promtool check config ops/prometheus/prometheus.dev.yml
   promtool check config ops/prometheus/prometheus.prod.yml
   promtool check rules ops/prometheus/rules/core.rules.yml
   promtool test rules ops/prometheus/tests/core.rules.test.yml
   ```
2. Install Python `${PYTHON_VERSION}` with `pip`, then:
   ```bash
   pip install ruff jsonschema yamllint
   yamllint ops/prometheus .github/workflows/obs3-prometheus-ci.yml
   ruff check scripts
   python scripts/obs3_verify_manifest.py --manifest ops/schemas/examples/prom_scrape.example.json --schema ops/schemas/manifest.schema.json
   ```
3. Install `shellcheck` (`brew install shellcheck` or `apt-get install shellcheck`) and run:
   ```bash
   shellcheck -S warning scripts/*.sh
   ```
4. For the anti-scan gates:
   ```bash
   grep -R -nE 'TODO|FIXME|PLACEHOLDER|CHANGEME|TBD|XXX' -- . ':!docs/CHANGELOG.md' ':!LICENSE'
   grep -R -nE '<<<<<<<|>>>>>>>' -- .
   ```

Keep the repository free of placeholders and conflict markers before pushing changes.
