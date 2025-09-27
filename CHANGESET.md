# CHANGESET — Fix-Pack Gate Hardening

## Summary
- Adiciona governança padrão Fix-Pack (`CODEOWNERS`, `SECURITY.md`, `.editorconfig`, `.dockerignore`, `.secrets.baseline`).
- Configura pipeline `ci.yml` com lint (`cargo fmt`/`cargo clippy`), testes, geração de SBOM (CycloneDX) e execução do gate A110 (`scripts/a110_run_invariants.sh`).
- Cria `docs-guard-agents.yml` para garantir que alterações em `AGENTS.md` dependam do selo `AGENTS-APPROVED`.

## Impacto Operacional
- Watchers passam a ter owners explícitos via CODEOWNERS.
- SBOM assinado e artefatos do gate ficam disponíveis como artifacts do workflow.
- Alterações em governança (`AGENTS.md`) exigem coordenação com SecOps/DocOps.

## Validação Manual
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all --all-features`
- `cargo install cargo-nextest --locked`
- `./scripts/a110_run_invariants.sh`
- `anchore/sbom-action@v0` via workflow (`trendmarket.sbom.cdx.json`)
