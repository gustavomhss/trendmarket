#!/usr/bin/env python3
"""Validate watcher specifications and emit a dry-run report."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any, Dict, List

REQUIRED_FIELDS = {"id", "domain", "owner", "kpi", "threshold", "window", "action"}


def _load_watchers(path: pathlib.Path) -> List[Dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(f"Watcher configuration not found: {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    watchers = data.get("watchers") if isinstance(data, dict) else None
    if not isinstance(watchers, list):
        raise ValueError("Watcher configuration must contain a 'watchers' list")
    return watchers


def _normalize_field(value: Any) -> Any:
    if isinstance(value, str):
        return value.strip()
    return value


def _validate_watcher(watcher: Dict[str, Any]) -> Dict[str, Any]:
    missing = REQUIRED_FIELDS - watcher.keys()
    if missing:
        raise ValueError(f"Watcher '{watcher.get('id')}' missing fields: {sorted(missing)}")
    normalized = {key: _normalize_field(watcher[key]) for key in sorted(REQUIRED_FIELDS)}
    encoded = json.dumps(normalized, sort_keys=True, ensure_ascii=False).encode("utf-8")
    digest = hashlib.sha256(encoded).hexdigest()
    return {
        "id": normalized["id"],
        "domain": normalized["domain"],
        "owner": normalized["owner"],
        "kpi": normalized["kpi"],
        "threshold": normalized["threshold"],
        "window": normalized["window"],
        "action": normalized["action"],
        "description": watcher.get("description", ""),
        "hash": digest,
    }


def generate_report(config: pathlib.Path) -> Dict[str, Any]:
    watchers = _load_watchers(config)
    validated = [_validate_watcher(w) for w in watchers]
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source": str(config),
        "total_watchers": len(validated),
        "watchers": validated,
    }


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args(argv)

    report = generate_report(args.config)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
