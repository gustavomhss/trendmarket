## Resumo
- Hardened `AmmError` into a stable contract with error codes, user messages, HTTP hints and descriptors, plus structured logging usage.
- Published the human catalog (`ops/errors/catalog_amm.yaml`), maintainer guide, QA index, and new contract regression test.
- Replaced tracked ZIP payloads with a reproducible packaging script so artifacts stay reproducible without blocking PR creation.

## Testes
- `cargo test`
- `rustfmt --edition 2021 --check src/amm/errors.rs src/bin/obs_demo.rs tests/amm_error_contract.rs`
