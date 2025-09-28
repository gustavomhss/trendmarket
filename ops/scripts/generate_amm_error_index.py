#!/usr/bin/env python3
"""Build the JSON index of AMM error codes from the YAML catalog."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
if str(SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(SCRIPT_DIR))

from config_loader import load_config

ROOT = SCRIPT_DIR.parent
CATALOG = ROOT / "errors" / "catalog_amm.yaml"
OUTPUT = ROOT / "reports" / "amm_error_index.json"

REQUIRED_META_KEYS = {"domain", "prefix", "version"}
REQUIRED_ENTRY_KEYS = {"variant", "code", "default_message", "http_status"}


def _load_catalog() -> dict[str, Any]:
    catalog_text = CATALOG.read_text(encoding="utf-8")
    data = load_config(catalog_text)
    if not isinstance(data, dict):
        raise SystemExit("catalog must be a YAML mapping")
    return data


def _normalize_meta(meta: Any) -> dict[str, Any]:
    if not isinstance(meta, dict):
        raise SystemExit("catalog meta section must be a mapping")

    missing_meta = REQUIRED_META_KEYS - meta.keys()
    if missing_meta:
        raise SystemExit(
            f"catalog meta missing required keys: {sorted(missing_meta)}"
        )

    normalized = dict(meta)
    try:
        normalized["version"] = int(meta["version"])
    except (TypeError, ValueError) as exc:  # pragma: no cover - defensive guard
        raise SystemExit(
            f"catalog meta version must be an integer, found {meta['version']!r}"
        ) from exc

    return normalized


def _normalize_errors(errors: Any) -> list[dict[str, Any]]:
    if not isinstance(errors, list) or not errors:
        raise SystemExit("catalog errors section must be a non-empty list")

    normalized_errors: list[dict[str, Any]] = []
    for index, entry in enumerate(errors, start=1):
        if not isinstance(entry, dict):
            raise SystemExit(
                f"catalog errors entry #{index} must be a mapping, found {type(entry).__name__}"
            )
        missing = REQUIRED_ENTRY_KEYS - entry.keys()
        if missing:
            raise SystemExit(
                "catalog errors entry #"
                f"{index} missing required keys {sorted(missing)}"
            )

        http_status_raw = entry["http_status"]
        if isinstance(http_status_raw, bool):
            # bool is a subclass of int; treat as invalid to avoid accidental truthy values.
            raise SystemExit(
                f"catalog errors entry #{index} has invalid boolean http_status"
            )
        try:
            http_status = int(http_status_raw)
        except (TypeError, ValueError) as exc:  # pragma: no cover - defensive guard
            raise SystemExit(
                f"catalog errors entry #{index} has non-integer http_status {http_status_raw!r}"
            ) from exc

        normalized_errors.append(
            {
                "variant": str(entry["variant"]),
                "code": str(entry["code"]),
                "message": str(entry["default_message"]),
                "http_status": http_status,
            }
        )

    return normalized_errors


def main() -> None:
    catalog = _load_catalog()
    if "meta" not in catalog:
        raise SystemExit("catalog missing meta section")
    if "errors" not in catalog:
        raise SystemExit("catalog missing errors section")

    meta = _normalize_meta(catalog["meta"])
    errors = _normalize_errors(catalog["errors"])

    payload = {"meta": meta, "errors": errors}
    OUTPUT.write_text(
        json.dumps(payload, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
