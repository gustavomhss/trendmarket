# ORR — Read-Only & Offline Operations

This note explains how to run the ORR (Operational Readiness Review) toolchain when the
repository checkout is mounted read-only or the workstation is offline.  The intent is to make
skips explicit, reproducible, and observable instead of silently failing mid-way through the
T1→T8 threads.

## 1. Environment probing (new helper)
- `scripts/orr_env_probe.sh` is a Bash 3.2 compatible probe that performs zero writes.
- The script prints a **single JSON line** with the repo root that was inspected, whether it is
  writable, and the availability of `bash`, `python3`, `jq`, and `gh`.
- Operators should run it before any ORR step:

  ```bash
  $ ./scripts/orr_env_probe.sh
  {"root":"/workspace/trendmarket","writable":true,"tools":{"bash":true,"python3":true,"jq":true,"gh":false}}
  ```

- When `"writable": false`, any step that would write under `out/orr_gatecheck/` **must terminate with
  exit code `95`** and print a short diagnostic that mirrors `READ-ONLY: skipping <step>`.
- The probe itself exits with `0` when it can report status, `1` when the repository root cannot be
  resolved, and never mutates the filesystem in read-only conditions.

## 2. Canonical exit codes

| Code | Meaning | Typical producer |
|------|---------|------------------|
| `0`  | Success, evidence generated | Every step (T1–T8) |
| `95` | Read-only/offline short-circuit (no writes attempted) | Wrapper invoking each ORR step |
| `2`  | Merge conflicts detected | `scripts/orr_t7_ci_prep.sh`, `scripts/orr_t8_bundle.sh` |
| `3`  | Placeholder token or authentication failure | `scripts/orr_t7_ci_prep.sh`, `scripts/orr_t6_metrics_run.sh`, `scripts/orr_t7_collect_ci.sh` |
| `4`  | Required evidence file missing or placeholder in metrics | `scripts/orr_t6_metrics_run.sh`, `scripts/orr_t8_bundle.sh` |
| `5`  | Packaging failure while generating the bundle | `scripts/orr_t8_bundle.sh` |
| `6`  | Bundle produced but ORR overall status is RED | `scripts/orr_t8_bundle.sh` |

> **Convention:** Exit `95` is reserved for deliberate read-only/offline skips so CI dashboards can
> distinguish infrastructure limitations from product failures. Any other unexpected condition should
> bubble up the tool's native exit code.

## 3. Expected STDOUT diagnostics by step

| Step | Command | Successful STDOUT excerpt | Read-only / Offline STDOUT |
|------|---------|---------------------------|----------------------------|
| **T1 — Unit** | `scripts/orr_t1_run.sh` | `running 128 tests`<br>`test amm::pricing::tests::apr_precision ... ok`<br>`test result: ok. 128 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` | `READ-ONLY: skipping T1 (out/orr_gatecheck)` |
| **T2 — Parse unit log** | `scripts/orr_t2_parse_unit.py` | `{"status": "GREEN", "passed": 128, "failed": 0, ...}` | `READ-ONLY: skipping T2 (log parsing requires writes)` |
| **T3 — Property tests** | `scripts/orr_t3_props_run.sh` | `running 32 tests`<br>`test property::amm_invariants::holds ... ok`<br>`status=GREEN failed=0` | `READ-ONLY: skipping T3 (unable to persist seeds)` |
| **T4 — Goldens** | `scripts/orr_t4_goldens_run.sh` | `Running golden assertions`<br>`golden::amm_cpmw_v1.csv ... ok` | `READ-ONLY: skipping T4 (golden diffs read-only)` |
| **T5 — Bench** | `scripts/orr_t5_bench_run.sh` | `running 4 tests`<br>`bench::swap ... bench: 45.123 µs/iter (+/- 0.800)` | `READ-ONLY: skipping T5 (criterion artifacts locked)` |
| **T6 — Metrics** | `scripts/orr_t6_metrics_run.sh` | `2025-01-05T12:00:00Z` (smoke timestamp) | `READ-ONLY: skipping T6 (metrics evidence)` |
| **T7 — CI** | `scripts/orr_t7_collect_ci.sh` | `gh run list --limit 20 ...` followed by JSON persisted under `out/orr_gatecheck/evidence/ci/run_summary.json` | `READ-ONLY: skipping T7 (CI metadata)` |
| **T8 — Bundle** | `scripts/orr_t8_bundle.sh` | `[2025-01-05T12:03:00Z] Higiene: conflitos e placeholders`<br>`[2025-01-05T12:03:01Z] Escrevendo ORR_README.md`<br>`[2025-01-05T12:03:07Z] T8 concluída — bundle pronto` | `READ-ONLY: skipping T8 (bundle cannot be generated)` |

The rightmost column shows the **exact diagnostic format** operators should emit before returning `95`
when the environment probe reports `"writable": false` or required tooling is missing. Offline
runs remain observable because every skip line is a single sentence that can be parsed by CI.

## 4. Operator workflow (offline scenario)
1. Run `./scripts/orr_env_probe.sh` and capture the JSON.
2. If `writable=false`, document the skip in the PR or ticket and short-circuit each ORR thread with
   `exit 95`, echoing the diagnostic from the table above.
3. Once a writable checkout is available, rerun the standard ORR threads (T1–T8); no additional
   cleanup is required because the offline attempt never touched the filesystem.

By codifying the probe + exit-code expectations, operators maintain auditability even when they can
only perform read-only investigations of the repository.
