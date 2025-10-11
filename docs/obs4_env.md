# OBS-4 Environment Bootstrap Guide

This document describes how to prepare the local environment and repository for the OBS-4 tracing gate-check with guardrails.

## Prerequisites

* macOS 15+ (zsh default shell) or Linux distributions with Bash 5+.
* Windows users should run within Windows Subsystem for Linux (WSL2 recommended).
* Python 3.9+ available on the PATH.
* `curl` **or** `wget` for downloading the OpenTelemetry Collector.

## Scripts Overview

### `scripts/obs4_env_bootstrap.sh`

This idempotent script prepares the local observability workspace without starting any background services.

**Responsibilities:**

1. Creates the directory structure under `out/obs_gatecheck/` (`logs`, `evidence`, `jira`, `tmp`, `diag`).
2. Ensures `.tools/` and `.venv/` are present and usable.
3. Installs/validates Python packages (`jsonschema`, `PyYAML`) inside the virtual environment. Offline snapshots are bundled
   under `vendor/python/` so the bootstrap works without external package mirrors.
4. Checks for `jq` (prints installation tips if missing).
5. Downloads `otelcol-contrib` v0.97.0 into `.tools/otelcol-contrib` when absent.
6. Sets default environment variables without overriding user-provided values:
   * `SERVICE_NAME=credit-engine-core`
   * `SERVICE_VERSION=0.0.0-dev`
   * `DEPLOY_ENV=dev`
   * `OTEL_TRACES_SAMPLER=parentbased_traceidratio`
   * `OTEL_TRACES_SAMPLER_ARG=0.1`
   * `OTEL_EXPORTER_OTLP_PROTOCOL=http/protobuf`
   * `OTELCOL_LISTEN_ADDR=127.0.0.1`
   * `OTELCOL_LISTEN_PORT=8888`
   * `OTLP_GRPC_PORT=4317`
   * `OTLP_HTTP_PORT=4318`
   * `TEMPO_HTTP_URL` or `JAEGER_HTTP_URL` are only exported if already set. A warning is logged if both are missing.
7. Performs port guard checks for 4317, 4318, and 8888 to avoid conflicts.
8. Cleans up orphan PID files located in `out/obs_gatecheck/logs/*.pid`.
9. Records an environment diagnostic snapshot at `out/obs_gatecheck/diag/obs4_env_<timestamp>.txt`.
10. Summarizes the verified paths at the end of the run.

**Usage:**

```bash
bash scripts/obs4_env_bootstrap.sh | tee out/obs_gatecheck/logs/obs4_env_bootstrap.out
```

Re-running the script is safe and will only perform actions when necessary.

### `scripts/obs4_branch_bootstrap.sh`

Creates a clean working branch for OBS-4 work.

**Responsibilities:**

1. Verifies the git working tree is clean; exits with code `2` if not.
2. Chooses `origin/main` as the base when available, otherwise uses the local `main` branch.
3. Checks out the base branch and creates a new branch named `obs4/tracing-<UTC timestamp>`.
4. Prints the new branch name in the format `BRANCH=obs4/tracing-...`.

**Usage:**

```bash
bash scripts/obs4_branch_bootstrap.sh
```

## Verification Commands

After running the bootstrap scripts, capture the verification output with:

```bash
bash scripts/obs4_env_bootstrap.sh
bash scripts/obs4_branch_bootstrap.sh
ls -la out/obs_gatecheck/{logs,evidence,jira,tmp,diag}
cat out/obs_gatecheck/diag/* | sed -n '1,80p'
```

Store the combined output in `out/obs_gatecheck/logs/obs4_thread01_verify.txt`.

## Troubleshooting

* **Ports already in use (4317/4318/8888):** Stop the conflicting process or adjust your local services before running the bootstrap script again. The script reports which ports are occupied.
* **Virtual environment issues:** If `.venv` becomes corrupted, delete it and rerun `scripts/obs4_env_bootstrap.sh` to recreate and reinstall dependencies.
* **Missing `jq`:** Install using your package manager. Examples: `brew install jq` (macOS), `sudo apt-get install jq` (Debian/Ubuntu), or `sudo dnf install jq` (Fedora). The script will continue but logs a warning.
* **Network restrictions:** Ensure outbound HTTPS access to `github.com` is available to download `otelcol-contrib`. If downloading manually, place the `otelcol-contrib` binary in `.tools/otelcol-contrib/` and rerun the script.

## Compatibility Notes

* Tested on macOS 15+ with the default `zsh` shell and Bash 5.1 via `/bin/bash`.
* Verified on Ubuntu 22.04 LTS.
* Windows users should rely on WSL2 to meet filesystem and tooling expectations.

Running these scripts prepares the repository for OBS-4 tasks without launching any services, ensuring a repeatable and conflict-free setup.
