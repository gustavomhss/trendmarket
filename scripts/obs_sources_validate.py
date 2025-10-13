from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict, List, Tuple

SOURCES_FILE = Path("ops/obs/sources.yaml")
EVIDENCE_PATH = Path("out/obs_gatecheck/evidence/t3_sources_validation.json")

EXPECTED_SOURCES: Dict[str, str] = {
    "oracle": "pricing",
    "market_feed": "market",
    "cdc_topic:orders": "cdc",
    "chain_header": "chain",
}


def parse_scalar(raw: str) -> Any:
    if raw == "":
        return ""
    if raw.isdigit():
        return int(raw)
    if raw.startswith("-") and raw[1:].isdigit():
        return int(raw)
    return raw


def simple_yaml_load(text: str) -> Dict[str, Any]:
    result: Dict[str, Any] = {}
    lines = text.splitlines()
    idx = 0
    current_entry: Dict[str, Any] | None = None
    sources: List[Dict[str, Any]] | None = None

    while idx < len(lines):
        line = lines[idx]
        if not line.strip():
            idx += 1
            continue
        if line.startswith("version:"):
            _, value = line.split(":", 1)
            result["version"] = parse_scalar(value.strip())
        elif line.startswith("sources:"):
            sources = []
            result["sources"] = sources
        elif line.startswith("  - "):
            if sources is None:
                raise ValueError("encountered list item before sources declaration")
            item_content = line[4:]
            current_entry = {}
            sources.append(current_entry)
            if item_content:
                key, value = item_content.split(":", 1)
                current_entry[key.strip()] = parse_scalar(value.strip())
        elif line.startswith("    "):
            if current_entry is None:
                raise ValueError("key-value without active list entry")
            stripped = line.strip()
            key, value = stripped.split(":", 1)
            current_entry[key.strip()] = parse_scalar(value.strip())
        else:
            raise ValueError(f"unsupported line format: {line}")
        idx += 1

    return result


def load_sources_config(path: Path) -> Dict[str, Any]:
    text = path.read_text(encoding="utf-8")
    try:
        import yaml  # type: ignore
    except ModuleNotFoundError:
        return simple_yaml_load(text)

    data = yaml.safe_load(text)
    if not isinstance(data, dict):
        raise ValueError("sources.yaml must contain a mapping")
    return data


def validate_rules(data: Dict[str, Any]) -> Tuple[bool, bool, int]:
    if "sources" not in data:
        raise ValueError("sources list is missing")
    sources_obj = data["sources"]
    if not isinstance(sources_obj, list):
        raise ValueError("sources must be a list")

    order_ok = True
    enums_ok = True
    seen_sources = set()

    for entry in sources_obj:
        if not isinstance(entry, dict):
            raise ValueError("each source entry must be a mapping")
        source_name = entry.get("source")
        domain_name = entry.get("domain")
        expected = entry.get("expected_interval_seconds")
        warn = entry.get("warn_seconds")
        crit = entry.get("crit_seconds")

        if source_name not in EXPECTED_SOURCES:
            enums_ok = False
        else:
            seen_sources.add(source_name)
            if EXPECTED_SOURCES[source_name] != domain_name:
                enums_ok = False

        if domain_name not in EXPECTED_SOURCES.values():
            enums_ok = False

        try:
            expected_val = int(expected)
            warn_val = int(warn)
            crit_val = int(crit)
        except (TypeError, ValueError):
            order_ok = False
            continue

        if not (crit_val >= warn_val >= expected_val):
            order_ok = False

    if len(sources_obj) != len(EXPECTED_SOURCES):
        enums_ok = False
    if seen_sources != set(EXPECTED_SOURCES.keys()):
        enums_ok = False

    return order_ok, enums_ok, len(sources_obj)


def write_evidence(payload: Dict[str, Any]) -> None:
    EVIDENCE_PATH.parent.mkdir(parents=True, exist_ok=True)
    with EVIDENCE_PATH.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")


def main() -> int:
    result = {
        "file": str(SOURCES_FILE),
        "count": 0,
        "rules": {"order": False, "enums": False},
        "ok": False,
    }

    if not SOURCES_FILE.exists():
        write_evidence(result)
        return 1

    try:
        data = load_sources_config(SOURCES_FILE)
        order_ok, enums_ok, count = validate_rules(data)
        result["count"] = count
        result["rules"] = {"order": order_ok, "enums": enums_ok}
        result["ok"] = order_ok and enums_ok
    except Exception as exc:
        print(f"validation error: {exc}", file=sys.stderr)
    finally:
        write_evidence(result)

    return 0 if result["ok"] else 1


if __name__ == "__main__":
    sys.exit(main())
