#!/usr/bin/env python3
"""Build the JSON index of AMM error codes from the YAML catalog."""
from __future__ import annotations

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "errors" / "catalog_amm.yaml"
OUTPUT = ROOT / "reports" / "amm_error_index.json"


def _strip_quotes(value: str) -> str:
    value = value.strip()
    if value.startswith("\"") and value.endswith("\""):
        return value[1:-1]
    return value


def extract_entries(text: str) -> list[dict[str, str]]:
    entries: list[dict[str, str]] = []
    current: dict[str, str] | None = None
    in_errors = False
    for raw_line in text.splitlines():
        if not raw_line.strip():
            continue
        if raw_line.startswith("errors:"):
            in_errors = True
            continue
        if not in_errors:
            continue
        if raw_line.startswith("  - "):
            if current:
                entries.append(current)
            current = {}
            line = raw_line.strip()
            key, value = line[len("- "):].split(":", 1)
            current[key.strip()] = _strip_quotes(value)
            continue
        if current is None:
            continue
        line = raw_line.strip()
        if ":" not in line:
            continue
        key, value = line.split(":", 1)
        current[key.strip()] = _strip_quotes(value)
    if current:
        entries.append(current)
    return entries


def main() -> None:
    catalog_text = CATALOG.read_text(encoding="utf-8")
    entries = extract_entries(catalog_text)
    index = [
        {
            "variant": entry["variant"],
            "code": entry["code"],
            "default_message": entry["default_message"],
        }
        for entry in entries
    ]
    OUTPUT.write_text(
        json.dumps(index, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
