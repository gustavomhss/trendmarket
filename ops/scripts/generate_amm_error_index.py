#!/usr/bin/env python3
"""Build the JSON index of AMM error codes from the YAML catalog."""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "errors" / "catalog_amm.yaml"
OUTPUT = ROOT / "reports" / "amm_error_index.json"


def _strip_quotes(value: str) -> str:
    value = value.strip()
    if value.startswith("\"") and value.endswith("\""):
        return value[1:-1]
    return value


def parse_meta(text: str) -> dict[str, str]:
    meta: dict[str, str] = {}
    in_meta = False
    for raw_line in text.splitlines():
        stripped = raw_line.strip()
        if not stripped:
            continue
        if stripped.startswith("#"):
            continue
        if stripped == "meta:":
            in_meta = True
            continue
        if stripped == "errors:":
            break
        if not in_meta:
            continue
        if not raw_line.startswith("  "):
            in_meta = False
            continue
        if ":" not in stripped:
            continue
        key, value = stripped.split(":", 1)
        meta[key.strip()] = _strip_quotes(value)
    return meta


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
    meta = parse_meta(catalog_text)
    missing_meta = {key for key in ("domain", "prefix", "version") if key not in meta}
    if missing_meta:
        raise SystemExit(f"catalog meta missing required keys: {sorted(missing_meta)}")

    try:
        version_value = int(meta["version"])
    except ValueError as exc:  # pragma: no cover - defensive guard
        raise SystemExit(
            f"catalog meta version must be an integer, found {meta['version']!r}"
        ) from exc

    normalized_meta: dict[str, Any] = {
        "domain": meta["domain"],
        "prefix": meta["prefix"],
        "version": version_value,
    }

    entries = extract_entries(catalog_text)
    if not entries:
        raise SystemExit("no error entries found in catalog")

    normalized_errors: list[dict[str, Any]] = []
    for entry in entries:
        required_keys = {"variant", "code", "default_message", "http_status"}
        missing = required_keys - entry.keys()
        if missing:
            raise SystemExit(f"entry {entry} missing keys {sorted(missing)}")
        try:
            http_status = int(entry["http_status"])
        except ValueError as exc:  # pragma: no cover - defensive guard
            raise SystemExit(
                f"http_status must be an integer for {entry['variant']}"
            ) from exc
        normalized_errors.append(
            {
                "variant": entry["variant"],
                "code": entry["code"],
                "message": entry["default_message"],
                "http_status": http_status,
            }
        )

    payload = {"meta": normalized_meta, "errors": normalized_errors}
    OUTPUT.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
   
