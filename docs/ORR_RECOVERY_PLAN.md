# ORR Recovery Plan (Read-Only Runner Incident)

## 1. Immediate Assessment
- The Codex report confirms the CI runner executed in a **read-only environment** with no shell utilities (`python`, `jq`, `gh`).
- All gate steps (T1–T8) therefore failed to materialise mandatory artefacts; failures are environmental, **not** due to business logic.
- Repo metadata matches the canonical repository, but Git status is absent because the workspace is not writable.

## 2. Engineering Directives
1. **No greenwashing** – Step T8 must refuse `GREEN` unless every required evidence file exists and is valid. Missing artefacts keep the run `RED` and increment the `kill` counter.
2. **Read-only detection** – Every runner (T1–T7) needs explicit read-only detection that exits with code `95` and emits a diagnostic JSON payload to `STDOUT` instead of writing to disk.
3. **Offline CI semantics** – Step T7 must detect missing `gh`/network capabilities, exit with code `2`, and only emit diagnostics to `STDOUT`; it must never fabricate `run_summary.json`.
4. **Resilient validation** – Step T8 should consume only existing, well-formed JSON artefacts, degrade gracefully when evidence is absent, and compute `overall=GREEN` **only** when every axis is `GREEN`.
5. **Atomic writes** – Any file creation must use `mkdir -p`, temporary files, and `mv` to guarantee atomicity. Never rely on placeholders.

## 3. Remediation Scope
- Modify only the runner scripts under `scripts/orr_t{1..8}_*.(sh|py)` plus the new helper `scripts/orr_env_probe.sh` and optional fixtures under `scripts/orr_dryrun_fixtures/`.
- Produce documentation in `docs/ORR_OFFLINE.md` that explains exit codes (`95`, `2`, `3`, etc.) and provides sample diagnostic outputs.
- Maintain macOS/Bash 3.2 compatibility (no `mapfile`, no non-portable substitutions).

## 4. Execution Strategy
1. Implement `scripts/orr_env_probe.sh` to report capability detection via JSON (write permissions, presence of `python3`, `jq`, `gh`, and `gh` auth state).
2. Update each runner:
   - **T1/T3/T4/T6** (shell): probe write access, create directories atomically, emit logs under `out/orr_gatecheck/logs/` when writable, otherwise exit `95` with `{ "step": "Tn", "error": "read_only" }`.
   - **T2/T5 collector/T8** (Python): wrap writes with parent directory creation plus atomic temp files; on read-only failures, print the would-be JSON to `STDOUT`, exit `95` (`T5` also uses exit `3` when `count==0`).
   - **T5 bench runner**: mirror read-only behaviour and exit `95` when blocking conditions occur.
   - **T7**: verify `gh` availability and authentication before running. Exit `2` with diagnostics when offline; exit `95` when the filesystem rejects writes.
3. Ensure the scripts never touch `src/**` and maintain compatibility with offline dry-run fixtures for static validation.
4. After implementing, the Codex must submit a PR describing:
   - Read-only/offline handling strategy.
   - Diagnostic JSON examples.
   - Assurance that T8 remains strict.

## 5. Local Validation Checklist (Writable Environment)
1. Run `bash scripts/orr_env_probe.sh` to confirm `writeable=true` and required tools installed/authenticated.
2. Execute each gate sequentially:
   ```bash
   bash scripts/orr_t1_run.sh
   python3 scripts/orr_t2_parse_unit.py
   bash scripts/orr_t3_props_run.sh
   bash scripts/orr_t4_goldens_run.sh || true
   python3 scripts/orr_t5_collect_criterion.py || { bash scripts/orr_t5_bench_run.sh; python3 scripts/orr_t5_collect_criterion.py; }
   bash scripts/orr_t6_metrics_run.sh
   bash scripts/orr_t7_collect_ci.sh
   python3 scripts/orr_t8_validate.py
   jq -r '.overall, .kill_criteria_count, (.exits|tojson)' out/orr_gatecheck/evidence/orr_final_summary.json
   ```
3. Acceptance conditions: `overall=GREEN`, `kill_criteria_count=0`, and every axis reports `GREEN`.

## 6. Rationale
- Running diagnostics without filesystem writes keeps the Codex workflow compliant with the read-only runner while preserving full transparency.
- Atomic writes and strict validation prevent partial artefacts and ensure integrity across reruns.
- The plan avoids masking missing evidence, preserving trust in ORR gates and ensuring that only genuine GREEN outcomes reach production.

