### Fix-Pack Gate Hardening

| Gate | Status | Detalhes |
| ---- | ------ | -------- |
| Lint | ⏳ | `cargo fmt --all -- --check` + `cargo clippy --all-targets --all-features -- -D warnings` |
| Testes | ⏳ | `cargo test --all --all-features` |
| Gate A110 | ⏳ | `./scripts/a110_run_invariants.sh` (instala `cargo-nextest`) |
| SBOM | ⏳ | `anchore/sbom-action@v0` → `trendmarket.sbom.cdx.json` |
| Docs Guard | ✅ | `docs-guard-agents.yml` exige label `AGENTS-APPROVED` para `AGENTS.md` |

> Atualize os status conforme os jobs rodarem na pipeline `ci`.
