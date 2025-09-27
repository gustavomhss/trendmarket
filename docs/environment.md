# Ambiente de Desenvolvimento e Reprodutibilidade

Este documento descreve o toolchain oficial, sementes determinísticas e práticas de reprodutibilidade
para o CreditEngine$ dentro do repositório `trendmarket`.

## Toolchain Canônico
- **Container base:** Ubuntu 22.04 + `uv` 0.4.x + `pnpm` 9.x + `docker` CLI 25.x.
- **Linguagens:** Python 3.11.8, Node.js 20.11, Rust 1.77 (para utilitários de infra), Go 1.22 (observabilidade).
- **Observabilidade:** OpenTelemetry Collector (`otelcol-contrib`), Prometheus, Grafana, Loki.
- **Data/ML:** dbt 1.7 (`profiles.yml` com target `creditengine`), Apache Iceberg com Catalog Glue compatível,
  Triton Inference Server 23.10, Great Expectations 0.18 para validações determinísticas.
- **CLI/Build:** `make`, `just`, `pre-commit` com hooks A110 habilitados (`watchers.dry`, `hooks.dry`).

Todos os binários são fornecidos via `sync_core_to_trendmarket.sh` e `docker-compose.bridge.yml`.

## Seeds Determinísticas
- **Dados sintéticos:** `goldens/decisions/*.parquet` gerados com seed global `CE_SEED=20240117`.
- **Modelos ML:** `ml/models/credit_decision_v5.onnx` com `MODEL_SHA=8b92fe4` e `MODEL_SEED=20231201`.
- **Experimentos:** `tests/ab/fixtures/*.json` com `AB_SEED=0xC0FFEE`, definidos em `Makefile` como `export SRM_SEED`.
- **Infra de observabilidade:** Dashboards Grafana versionadas via `ops/grafana/dashboards/*.json` com checksum SHA256
  registrado em `ops/reports/repo_audit.json`.

As sementes são expostas como variáveis de ambiente no arquivo `.env` gerado por `make env.up`, garantindo reprodutibilidade
em pipelines CI/CD e ambientes locais.

## Garantias de Reprodutibilidade
1. **Lockfiles obrigatórios:** `uv.lock`, `pnpm-lock.yaml`, `Cargo.lock` e `dbt_packages.yml` versionados.
2. **Pipelines determinísticos:** Jobs CI executam `uv sync --frozen` e `pnpm install --frozen-lockfile` com variáveis de seed fixas.
3. **Watchers A110:** `runtime_eol_watch` e `model_drift_watch` verificam deriva de ambiente.
4. **Auditoria:** `ops/reports/repo_audit.json` inclui rastreabilidade de toolchain e seeds, com owners e data de expiração.

Qualquer alteração no ambiente deve atualizar este documento, `docs/PRFAQ.md` (seção "What changed in the environment?") e o
relatório de auditoria.
