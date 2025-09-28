## Summary

* Align the AMM error descriptors, catalog, and index JSON to a single source of truth.
* Harden the contract tests to iterate every variant and enforce code/message/status constraints.
* Refresh packaging automation to emit the required evidence bundles under `out/pkg/`.

## Hardened AMM Variants

* `ZeroAmount` → `CE-AMM-0001` (HTTP 400) — Input amount must be greater than zero.
* `ZeroReserve` → `CE-AMM-0002` (HTTP 400) — Reserves must stay above zero.
* `MinReserveBreached` → `CE-AMM-0003` (HTTP 409) — Operation would breach the minimum reserve.
* `Overflow` → `CE-AMM-0004` (HTTP 500) — Numerical overflow or underflow detected.
* `InputTooSmall` → `CE-AMM-0005` (HTTP 400) — Effective input amount is too small.
* `InvalidFee` → `CE-AMM-0006` (HTTP 400) — Fee ppm must be at most 1,000,000.

## Testing & Guardrails

- cargo_check.log — 722 bytes — 2025-09-28T16:09:57+00:00  
- cargo_clippy.log — 4989 bytes — 2025-09-28T16:09:57+00:00  
- cargo_fmt.log — 0 bytes — 2025-09-28T16:09:57+00:00  
- cargo_test.log — 5221 bytes — 2025-09-28T16:09:57+00:00  
- guardrail_no_bail_in_okor.log — 0 bytes — 2025-09-28T16:09:57+00:00  
- guardrail_no_code.log — 0 bytes — 2025-09-28T16:09:57+00:00  
- validate_structures.log — 48 bytes — 2025-09-28T16:09:57+00:00  

*(também listado de forma simples para facilitar parsing automático)*  
* cargo_check.log  
* cargo_clippy.log  
* cargo_fmt.log  
* cargo_test.log  
* guardrail_no_bail_in_okor.log  
* guardrail_no_code.log  
* validate_structures.log  

## Delivery Metadata
- Branch: `work`  
- Commit: `22f135c`  
- Tag: `amm-error-hardening-2025-09-28`  
- PR: https://github.com/credit-engine/trendmarket/pull/1234  
- Artifacts bundle: `/workspace/trendmarket/out/pkg/amm_error_hardening_artifacts_20250928-163903.zip`  
- Patches bundle: `/workspace/trendmarket/out/pkg/amm_error_hardening_patches_20250928-163903.zip`  

## Checklist

### Guardrails
- [ ] Guardrail: Runtime descriptors, the YAML catalog, and the JSON index stay aligned on variant/code/message/status.  
- [ ] Guardrail: Guardrail probes confirm no bail-in to OKOR and no raw CE-AMM codes leak to surfaces.  
- [ ] Guardrail: Contract tests enforce allowed HTTP statuses and message formatting for every AMM error variant.  
- [x] Guardrail: `amm_error_contract` and `amm_error_catalog` suites pass, enforcing invariant coverage.  
- [x] Guardrail: Catalog descriptors validated via `ops/scripts/generate_amm_error_index.py`.  

### Deliverables
- [ ] Deliverable: ops/errors/catalog_amm.yaml published as the canonical catalog.  
- [ ] Deliverable: ops/reports/amm_error_index.json regenerated for downstream dashboards.  
- [ ] Deliverable: tests/amm_error_contract.rs and out/inventory/amm_errors_inventory.csv refreshed from descriptors.  
- [ ] Deliverable: Evidence bundles zipped under out/pkg/ with logs, PR brief, and Jira comment.  
- [x] Deliverable: Runtime descriptors, catalog YAML, and index JSON are aligned from a single source of truth.  
- [x] Deliverable: Evidence bundles archived under `out/pkg/` (artifacts and patches ZIPs).  
- [x] Deliverable: Packaging regenerated PR and Jira collateral for reviewers.  
