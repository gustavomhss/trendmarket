#!/usr/bin/env python3
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVI_DIR = ROOT / "out" / "obs_gatecheck" / "evidence"
SAMPLE_PATH = EVI_DIR / "traces_sample.json"
RAW_PATH = EVI_DIR / "traces_raw.json"

if not SAMPLE_PATH.exists():
    print("ACCEPTANCE_FAIL: traces_sample.json não encontrado")
    sys.exit(3)

try:
    sample = json.loads(SAMPLE_PATH.read_text(encoding="utf-8"))
except json.JSONDecodeError as err:
    print(f"ACCEPTANCE_FAIL: traces_sample.json inválido ({err})")
    sys.exit(3)

required_keys = ["trace_ok", "slow_captured", "error_captured", "links_cdc_amm"]
for key in required_keys:
    if key not in sample:
        print(f"ACCEPTANCE_FAIL: chave ausente: {key}")
        sys.exit(3)
    if not isinstance(sample[key], bool):
        print(f"ACCEPTANCE_FAIL: valor de {key} não é booleano")
        sys.exit(3)

if RAW_PATH.exists():
    try:
        raw_data = json.loads(RAW_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError as err:
        print(f"ACCEPTANCE_FAIL: traces_raw.json inválido ({err})")
        sys.exit(3)

    counters = {"traces": 0, "error": False}

    def walk(node):
        if isinstance(node, dict):
            if any(k in node for k in ("traceId", "traceID")):
                counters["traces"] += 1
            status = node.get("status")
            if isinstance(status, dict):
                code = status.get("code") or status.get("Code") or status.get("codeString")
                if isinstance(code, str) and "ERROR" in code.upper():
                    counters["error"] = True
                elif isinstance(code, (int, float)) and code != 0:
                    counters["error"] = True
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for item in node:
                walk(item)

    walk(raw_data)

    if counters["traces"] == 0:
        print("ACCEPTANCE_FAIL: traces_raw.json sem traces")
        sys.exit(3)
    if not counters["error"]:
        print("ACCEPTANCE_FAIL: nenhuma ocorrência de status ERROR em traces_raw.json")
        sys.exit(3)

print("ACCEPTANCE_OK")
