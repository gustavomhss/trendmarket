#!/usr/bin/env python3
"""Build the JSON index of AMM error codes from the YAML catalog."""
from __future__ import annotations

import json
from pathlib import Path

try:
    import yaml
except ImportError as exc:  # pragma: no cover - import guard
    raise SystemExit(
        "PyYAML is required to generate the AMM error index. Install it with `pip install PyYAML`."
    ) from exc

ROOT = Path(__file__).resolve().parents[1]
CATALOG = ROOT / "errors" / "catalog_amm.yaml"
OUTPUT = ROOT / "reports" / "amm_error_index.json"


def main() -> None:
    catalog_data = yaml.safe_load(CATALOG.read_text(encoding="utf-8"))

    meta = catalog_data.get("meta", {})
    errors = catalog_data.get("errors", [])

    index = {
        "meta": {
            "domain": meta.get("domain", "AMM"),
            "prefix": meta.get("prefix", "CE-AMM"),
            "version": meta.get("version", 1),
        },
        "errors": [
            {
                "variant": entry["variant"],
                "code": entry["code"],
                "message": entry["default_message"],
                "http_status": entry["http_status"],
            }
            for entry in errors
        ],
    }

    OUTPUT.write_text(
        json.dumps(index, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
