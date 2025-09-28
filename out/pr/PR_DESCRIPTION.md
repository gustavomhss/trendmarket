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
- cargo_check.log — 722 bytes — 2025-09-28T16:09:57+00:00
- cargo_clippy.log — 4989 bytes — 2025-09-28T16:09:57+00:00
- cargo_fmt.log — 0 bytes — 2025-09-28T16:09:57+00:00
- cargo_test.log — 5221 bytes — 2025-09-28T16:09:57+00:00
- guardrail_no_bail_in_okor.log — 0 bytes — 2025-09-28T16:09:57+00:00
- guardrail_no_code.log — 0 bytes — 2025-09-28T16:09:57+00:00
- validate_structures.log — 48 bytes — 2025-09-28T16:09:57+00:00

## Delivery Metadata
- Branch: `work`
- Commit: `22f135c`
- Tag: `amm-error-hardening-2025-09-28`
- PR: https://github.com/credit-engine/trendmarket/pull/1234
- Artifacts bundle: `/workspace/trendmarket/out/pkg/amm_error_hardening_artifacts_20250928-163903.zip`
- Patches bundle: `/workspace/trendmarket/out/pkg/amm_error_hardening_patches_20250928-163903.zip`

