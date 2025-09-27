# Repo Audit — Governance Hooks

## Gates Registrados
- **CI (lint/test)** — Workflow `.github/workflows/ci.yml`, jobs `lint` e `test` executam `cargo fmt`, `cargo clippy`, `cargo test`.
- **Gate A110** — Workflow `.github/workflows/ci.yml`, job `gate_a110` executa `scripts/a110_run_invariants.sh` e publica artifacts.
- **SBOM** — Workflow `.github/workflows/ci.yml`, job `sbom` gera CycloneDX via `anchore/sbom-action@v0` (`trendmarket.sbom.cdx.json`).
- **Docs Guard** — Workflow `.github/workflows/docs-guard-agents.yml` bloqueia alterações em `AGENTS.md` sem label `AGENTS-APPROVED`.

## Controles de Suporte
- `CODEOWNERS` define owners para workflows, ADRs e scripts críticos.
- `.secrets.baseline` registra detectores ativos (detect-secrets 1.5.0) — revisar a cada sprint.
- `.dockerignore` reduz superfície de build; revisar se novos diretórios críticos forem criados.
- `SECURITY.md` formaliza contato (`security@creditengine.com`) e SLA de resposta.

## Próximos Passos
- Automatizar upload da SBOM para o registro central (S3 `security-artifacts/trendmarket/`).
- Adicionar verificação de assinatura para releases do collector.
