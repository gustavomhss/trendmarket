#!/usr/bin/env bash
set -Eeuo pipefail

summary_dir="out/obs_gatecheck/evidence"
mkdir -p "$summary_dir"

step_names=("unit_tests" "sources_validate" "recording_apply" "smoke")
step_scripts=(
  "scripts/obs_t6_tests.sh"
  "scripts/obs_t6_sources_validate.sh"
  "scripts/obs_t6_recording_apply.sh"
  "scripts/obs_t6_staleness_smoke.sh"
)
step_tokens=("TESTS_OK" "SOURCES_OK" "RECORDING_OK" "STALE_SMOKE_OK")

json_steps=()
for i in "${!step_names[@]}"; do
  name="${step_names[$i]}"
  script="${step_scripts[$i]}"
  expected="${step_tokens[$i]}"
  if ! output="$($script)"; then
    printf 'TESTS_FAIL:%s\n' "$name"
    exit 6
  fi
  if ! printf '%s\n' "$output" | grep -q "$expected"; then
    printf 'TESTS_FAIL:%s\n' "$name"
    exit 6
  fi
  json_steps+=("$(printf '{"name": "%s", "token": "%s", "ok": true}' "$name" "$expected")")
done

summary_file="$summary_dir/t8_tests_summary.json"
{
  printf '{\n'
  printf '  "obs": "OBS-6",\n'
  printf '  "thread": "T8",\n'
  printf '  "steps": [\n'
  for i in "${!json_steps[@]}"; do
    if [ "$i" -gt 0 ]; then
      printf ',\n'
    fi
    printf '    %s' "${json_steps[$i]}"
  done
  printf '\n'
  printf '  ],\n'
  printf '  "ok": true\n'
  printf '}\n'
} > "$summary_file"

printf 'TESTS_OK\n'
