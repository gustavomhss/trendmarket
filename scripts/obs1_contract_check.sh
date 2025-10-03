#!/usr/bin/env bash
set -euo pipefail

ROOT=$(git rev-parse --show-toplevel)
cd "$ROOT"

declare -a TARGETS=(
  "docs/obs1_contract.md"
  "schemas/obs1_log_record.schema.json"
  "schemas/obs1_contract.yaml"
  "src/telemetry_contract.rs"
  "out/obs_gatecheck/docs/OBS1_CONTRACT_README.md"
  "out/obs_gatecheck/evidence/obs1_contract_report.json"
)

if rg -n "TBD|FIXME|…|PLACEHOLDER" "${TARGETS[@]}"; then
  echo "Forbidden token detected" >&2
  exit 1
fi

python - <<'PY'
import json, re
from pathlib import Path

schema = json.loads(Path('schemas/obs1_log_record.schema.json').read_text())
required = set(schema['required'])
properties = schema['properties']

valid_example = {
    "ts": "2025-10-03T12:34:56Z",
    "level": "info",
    "msg": "swap executed",
    "trace_id": "4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f",
    "span_id": "9a3b7c1d2e3f4a5b",
    "service": "ce-amm",
    "env": "dev",
    "op": "swap",
    "version": "1.2.3",
    "hook_id": "risk-check",
    "extra": {"amm": {"k_before": 1.0, "k_after": 1.05, "delta_k_ratio": 0.05, "fee_ppm": 300}}
}

invalid_example = {
    "ts": "2025-10-03T12:34:56Z",
    "level": "info",
    "msg": "swap",
    "trace_id": "4fd0c2...",
    "span_id": "9a3b7c...",
    "service": "ce-amm",
    "env": "dev",
    "op": "swap",
    "version": "1.2.3",
    "email": "cliente@exemplo.com"
}

pattern_cache = {}

def check(instance):
    keys = set(instance)
    missing = required - keys
    if missing:
        raise SystemExit(f"missing required fields: {sorted(missing)}")
    extra = keys - set(properties)
    if extra:
        raise SystemExit(f"unexpected fields: {sorted(extra)}")
    for key, value in instance.items():
        spec = properties[key]
        if spec.get('type') == 'string':
            if not isinstance(value, str):
                raise SystemExit(f"{key} must be string")
            pattern = spec.get('pattern')
            if pattern:
                compiled = pattern_cache.setdefault(pattern, re.compile(pattern))
                if not compiled.fullmatch(value):
                    raise SystemExit(f"{key} pattern mismatch: {value}")
            enum = spec.get('enum')
            if enum and value not in enum:
                raise SystemExit(f"{key} invalid enum value: {value}")
        elif spec.get('type') == 'object':
            if not isinstance(value, dict):
                raise SystemExit(f"{key} must be object")
            name_pattern = spec.get('propertyNames', {}).get('pattern')
            if name_pattern:
                compiled = pattern_cache.setdefault(name_pattern, re.compile(name_pattern))
                for child_key in value:
                    if not compiled.fullmatch(child_key):
                        raise SystemExit(f"{key}.{child_key} violates name policy")
    for clause in schema.get('allOf', []):
        neg = clause.get('not')
        if not neg:
            continue
        for condition in neg.get('anyOf', []):
            req = condition.get('required')
            if req and all(field in instance for field in req):
                raise SystemExit(f"PII field present: {req}")

check(valid_example)
try:
    check(invalid_example)
except SystemExit:
    pass
else:
    raise SystemExit('invalid example should fail')
PY

echo "OBS-1 contract checks passed."
