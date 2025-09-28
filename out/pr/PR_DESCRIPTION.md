## Summary
    - Align the AMM error descriptors, catalog, and index JSON to a single source of truth.
    - Harden the contract tests to iterate every variant and enforce code/message/status constraints.
    - Refresh packaging automation to emit the required evidence bundles under out/pkg/.

    ## Hardened AMM Variants
    - `ZeroAmount` → `CE-AMM-0001` (HTTP 400) — Input amount must be greater than zero.
- `ZeroReserve` → `CE-AMM-0002` (HTTP 400) — Reserves must stay above zero.
- `MinReserveBreached` → `CE-AMM-0003` (HTTP 409) — Operation would breach the minimum reserve.
- `Overflow` → `CE-AMM-0004` (HTTP 500) — Numerical overflow or underflow detected.
- `InputTooSmall` → `CE-AMM-0005` (HTTP 400) — Effective input amount is too small.
- `InvalidFee` → `CE-AMM-0006` (HTTP 400) — Fee ppm must be at most 1,000,000.

    ## Testing & Guardrails
    - cargo_test.log
- rustfmt.log

## Checklist
- [x] Guardrail: `amm_error_contract` and `amm_error_catalog` suites pass, enforcing invariant coverage.
- [x] Guardrail: Catalog descriptors validated via `ops/scripts/generate_amm_error_index.py`.
- [x] Deliverable: Runtime descriptors, catalog YAML, and index JSON are aligned from a single source of truth.
- [x] Deliverable: Evidence bundles archived under `out/pkg/` (artifacts and patches ZIPs).
- [x] Deliverable: Packaging regenerated PR and Jira collateral for reviewers.
