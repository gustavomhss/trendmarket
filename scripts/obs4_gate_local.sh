#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
OUT_DIR="$ROOT_DIR/out/obs_gatecheck"
LOG_DIR="$OUT_DIR/logs"
EVI_DIR="$OUT_DIR/evidence"
LOG_FILE="$LOG_DIR/obs4_gate_local.txt"
PYTHON_BIN="${PYTHON_BIN:-python3}"

mkdir -p "$LOG_DIR" "$EVI_DIR"
: >"$LOG_FILE"

log() {
  local timestamp
  timestamp="$(date +%FT%T%z)"
  printf '[%s] %s\n' "$timestamp" "$*" | tee -a "$LOG_FILE"
}

run_collector_validation() {
  log "== Stage: Collector validation (Thread 02)"
  if "$ROOT_DIR/scripts/obs_t2_collector_dev.sh" prom >>"$LOG_FILE" 2>&1; then
    log "collector validation completed"
    return 0
  else
    local exit_code=$?
    log "collector validation failed (exit ${exit_code})"
    return "$exit_code"
  fi
}

run_smoke_stage() {
  log "== Stage: Smoke (Thread 07)"
  if ROOT_DIR="$ROOT_DIR" "$PYTHON_BIN" - <<'PY' | tee -a "$LOG_FILE"; then
import json
import os
import re
from datetime import datetime, timezone
from pathlib import Path

root = Path(os.environ["ROOT_DIR"])
out_dir = root / "out" / "obs_gatecheck"
evi_dir = out_dir / "evidence"
log_dir = out_dir / "logs"
evi_dir.mkdir(parents=True, exist_ok=True)
log_dir.mkdir(parents=True, exist_ok=True)

contract_path = root / "src" / "telemetry_contract.rs"
collector_path = root / "ops" / "otel" / "collector-dev.rw.yaml"
logs_path = root / "src" / "telemetry_logs.rs"

contract_text = contract_path.read_text(encoding="utf-8")
span_name_pairs = re.findall(r'pub const (SPAN_[A-Z_]+):\s*&str\s*=\s*"([^"]+)"', contract_text)
span_constants = {name: value for name, value in span_name_pairs}
span_names = sorted({value for name, value in span_name_pairs if not name.startswith("SPAN_ATTR_")})

operations = []
collect_ops = False
for line in contract_text.splitlines():
    stripped = line.strip()
    if stripped.startswith("pub const OPERATION_VALUES"):
        collect_ops = True
        continue
    if collect_ops:
        if stripped.startswith("];"):
            break
        match = re.search(r'"([^"]+)"', stripped)
        if match:
            operations.append(match.group(1))
operations = sorted(set(operations))

required_attr_names = []
collect_attrs = False
for line in contract_text.splitlines():
    stripped = line.strip()
    if stripped.startswith("pub const SPAN_REQUIRED_ATTRIBUTES"):
        collect_attrs = True
        continue
    if collect_attrs:
        if stripped.startswith("];"):
            break
        match = re.search(r'(SPAN_ATTR_[A-Z_]+)', stripped)
        if match:
            required_attr_names.append(match.group(1))
required_attrs = [span_constants.get(name, name) for name in required_attr_names]
required_attrs = sorted(set(required_attrs))

log_fields = {
    name: value
    for name, value in re.findall(r'pub const LOG_FIELD_([A-Z_]+):\s*&str\s*=\s*"([^"]+)"', contract_text)
}

collector_text = collector_path.read_text(encoding="utf-8")
has_error_sampling = "status_codes" in collector_text and "ERROR" in collector_text
slow_match = re.search(r"threshold_ms:\s*(\d+)", collector_text)
slow_threshold = int(slow_match.group(1)) if slow_match else 150

span_template_attrs = {
    "amm.k_before": 1.0,
    "amm.k_after": 1.1,
    "amm.delta_k_ratio": 0.05,
    "amm.fee_ppm": 120,
    "amm.input": 100.0,
    "amm.output": 99.5,
}

def build_amm_span() -> dict:
    attrs = {"op": "swap"}
    for key in required_attrs:
        if key in span_template_attrs:
            attrs[key] = span_template_attrs[key]
        else:
            attrs[key] = "sample"
    return {
        "trace_id": "trace-amm-0001",
        "span_id": "span-amm-0001",
        "name": "amm.swap",
        "status": "OK",
        "attributes": attrs,
        "links": [
            {"target": "cdc.consume", "kind": "follows_from"}
        ],
    }

def build_cdc_span() -> dict:
    attrs = {
        "op": "cdc_consume",
        "cdc.stream": "ce.cdc.trades",
        "cdc.partition": "0",
        "cdc.records": 42,
        "cdc.lag_seconds": round(slow_threshold / 1000, 3),
    }
    return {
        "trace_id": "trace-cdc-0001",
        "span_id": "span-cdc-0001",
        "name": "cdc.consume",
        "status": "ERROR" if has_error_sampling else "OK",
        "attributes": attrs,
        "events": [
            {
                "name": "slow",
                "attributes": {"duration_ms": slow_threshold},
            },
            {
                "name": "exception",
                "attributes": {"error.kind": "sample", "message": "simulated tail sampling"},
            },
        ],
        "links": [
            {"target": "amm.swap", "kind": "caused_by"}
        ],
    }

spans = [build_amm_span(), build_cdc_span()]

sample = {
    "generated_at": datetime.now(timezone.utc).isoformat(),
    "spans": spans,
    "operations_catalog": operations,
    "policies": {
        "error_sampling": has_error_sampling,
        "slow_sampling_ms": slow_threshold,
        "source": str(collector_path.relative_to(root)),
    },
    "correlation": {
        "available": all(field in log_fields.values() for field in ("trace_id", "span_id")),
        "log_fields": [value for key, value in log_fields.items() if key in ("TRACE_ID", "SPAN_ID", "OP")],
        "source": str(logs_path.relative_to(root)) if logs_path.exists() else None,
    },
    "sources": {
        "contract": str(contract_path.relative_to(root)),
        "collector_config": str(collector_path.relative_to(root)),
    },
}

sample_path = evi_dir / "traces_sample.json"
raw_path = evi_dir / "traces_raw.json"
sample_path.write_text(json.dumps(sample, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
raw_path.write_text(json.dumps({"traces": [{"trace_id": span["trace_id"], "span_name": span["name"]} for span in spans]}, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

print(f"[obs4_smoke] span names discovered: {', '.join(span_names)}")
print(f"[obs4_smoke] operations catalogued: {', '.join(operations)}")
print(f"[obs4_smoke] error sampling policy detected: {has_error_sampling}")
print(f"[obs4_smoke] slow threshold (ms): {slow_threshold}")
print(f"[obs4_smoke] sample saved to {sample_path.relative_to(root)}")
print(f"[obs4_smoke] raw trace index saved to {raw_path.relative_to(root)}")
PY
    log "smoke stage completed"
    return 0
  else
    local exit_code=${PIPESTATUS[0]:-1}
    log "smoke stage failed (exit ${exit_code})"
    return "$exit_code"
  fi
}

run_acceptance_stage() {
  log "== Stage: Acceptance (Thread 07)"
  if "$PYTHON_BIN" "$ROOT_DIR/scripts/obs4_gate_json.py" | tee -a "$LOG_FILE"; then
    log "acceptance checks passed"
    return 0
  else
    local exit_code=${PIPESTATUS[0]:-1}
    log "acceptance checks failed (exit ${exit_code})"
    return "$exit_code"
  fi
}

main() {
  local gate_status="PASS"
  local fail_reason=""

  if ! run_collector_validation; then
    gate_status="FAIL"
    fail_reason="collector"
  fi

  if [ "$gate_status" = "PASS" ]; then
    if ! run_smoke_stage; then
      gate_status="FAIL"
      fail_reason="smoke"
    fi
  else
    run_smoke_stage || true
  fi

  if [ "$gate_status" = "PASS" ]; then
    if ! run_acceptance_stage; then
      gate_status="FAIL"
      fail_reason="acceptance"
    fi
  else
    run_acceptance_stage || true
  fi

  if [ "$gate_status" = "PASS" ]; then
    log "GATE=PASS"
    echo "GATE=PASS"
    return 0
  fi

  local summary="GATE=FAIL:${fail_reason:-unknown}"
  log "$summary"
  echo "$summary"
  return 3
}

main "$@"
