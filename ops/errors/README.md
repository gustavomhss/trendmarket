# AMM Error Contract Guide

This guide documents how to evolve the AMM error contract safely. The contract is
consumed by UI, API, and observability surfaces and **must remain stable**.

## Adding a new error variant

1. **Declare the variant** in `src/amm/errors.rs` and keep the enum payload-free.
2. **Assign contract metadata**:
   - Extend the `AmmError::ALL_VARIANTS` array with the new variant.
   - Provide mappings in `error_code()`, `user_message()`, `http_status()`, and `variant_name()`.
   - Codes must follow `CE-AMM-XXXX`, be unique, and never be reused.
   - Messages are short, neutral English sentences ending with a period.
3. **Update the catalog** in `ops/errors/catalog_amm.yaml` with the same metadata.
4. **Refresh the QA index** in `ops/reports/amm_error_index.json` (sorted by code).
5. **Extend the tests**:
   - Update assertions in `tests/amm_error_contract.rs` if new validation is required.
   - Ensure `variant_count::<AmmError>()` matches the number of variants.
6. **Document evidence**: run `ops/scripts/package_amm_error_contract.sh` to
   regenerate the artifacts bundle so the formatter, linter, and test logs
   capture the new variant.

## Validating changes locally

```bash
cargo fmt
cargo clippy --all-targets --all-features
cargo test
```

Keep the logs for these commands; they are bundled in the artifacts ZIP.

## Releasing the change

1. Execute `ops/scripts/package_amm_error_contract.sh` (optionally passing the
   merge base) to refresh logs, artifacts, and patch outputs under `out/`.
2. The script writes the Git diff to `out/patches/amm_error_contract.patch` and
   produces ZIP archives (ignored by Git) for hand-off when required.
3. Tag the commit with an **annotated** tag (`git tag -a`).
4. Update the Jira comment template in `_jira_out/amm_error_contract.txt`.

Each release should be auditable: the catalog, QA index, tests, and logs must be
present so Support, Ops, and integrators can rely on the error surface. The
packaging script keeps the reproducible ZIPs outside version control so they do
not block PR creation while remaining one command away for distribution.
