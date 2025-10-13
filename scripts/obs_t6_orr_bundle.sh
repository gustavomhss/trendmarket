#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="out/obs_gatecheck"
if [ ! -d "$ROOT" ] || [ ! -d "$ROOT/logs" ] || [ ! -d "$ROOT/evidence" ]; then
  echo "BUNDLE_FAIL:missing"
  exit 11
fi
TS="$(date -u +%Y%m%dT%H%M%SZ)"
BUNDLE="out/obs_gatecheck_bundle_${TS}.zip"
if ! command -v zip >/dev/null 2>&1; then
  echo "BUNDLE_FAIL:zip"
  exit 12
fi
if ! (cd out && zip -qr "$(basename "$BUNDLE")" obs_gatecheck); then
  echo "BUNDLE_FAIL:zip"
  exit 12
fi
if command -v sha256sum >/dev/null 2>&1; then
  SHA="$(sha256sum "$BUNDLE" | awk '{print $1}')"
elif command -v shasum >/dev/null 2>&1; then
  SHA="$(shasum -a 256 "$BUNDLE" | awk '{print $1}')"
else
  echo "BUNDLE_FAIL:sha"
  exit 13
fi
if [ -z "$SHA" ]; then
  echo "BUNDLE_FAIL:sha"
  exit 13
fi
printf "%s  %s\n" "$SHA" "$(basename "$BUNDLE")" > "$ROOT/bundle.sha256.txt"
expected_tokens=("CARDINALITY_OK" "COLLECTOR_HEALTH_OK" "PROM_READY_OK" "RECORDING_OK" "STALE_SMOKE_OK" "TESTS_OK")
tokens_found=()
if ls "$ROOT/logs"/*.txt >/dev/null 2>&1; then
  matches="$(grep -hFxf <(printf '%s\n' "${expected_tokens[@]}") "$ROOT"/logs/*.txt || true)"
  if [ -n "$matches" ]; then
    mapfile -t tokens_found < <(printf '%s\n' "$matches" | sort -u)
  fi
fi
ok_value=false
if [ "${#tokens_found[@]}" -eq "${#expected_tokens[@]}" ]; then
  ok_value=true
fi
mapfile -t files_list < <(find "$ROOT" -type f -print | sed "s#^$ROOT/##" | sort)
json_array() {
  local arr=("$@")
  local out="["
  local first=1
  for item in "${arr[@]}"; do
    local esc="${item//\\/\\\\}"
    esc="${esc//\"/\\\"}"
    if [ $first -eq 0 ]; then
      out+=","
    fi
    out+="\"$esc\""
    first=0
  done
  out+="]"
  printf '%s' "$out"
}
files_json="$(json_array "${files_list[@]}")"
tokens_json="$(json_array "${tokens_found[@]}")"
manifest_path="$ROOT/evidence/orr_manifest.json"
cat > "$manifest_path" <<JSON
{
  "obs": "OBS-6",
  "thread": "T9",
  "bundle": "${BUNDLE}",
  "sha256": "${SHA}",
  "files": ${files_json},
  "tokens": ${tokens_json},
  "ok": ${ok_value}
}
JSON
pr_body_path="$ROOT/evidence/orr_pr_body.txt"
token_line="none"
if [ "${#tokens_found[@]}" -gt 0 ]; then
  token_line="${tokens_found[0]}"
  for ((i=1;i<${#tokens_found[@]};i++)); do
    token_line+="; ${tokens_found[$i]}"
  done
fi
cat > "$pr_body_path" <<PR
## OBS-6 ORR Closure

- Bundle: ${BUNDLE}
- SHA-256: ${SHA}
- Files collected: ${#files_list[@]}
- Tokens: ${token_line}
PR
echo "BUNDLE_OK"
exit 0
