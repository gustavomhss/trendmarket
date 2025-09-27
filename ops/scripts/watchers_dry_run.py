#!/usr/bin/env python3
"""Validate watcher specifications and emit a dry-run report."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any, Dict, List

REQUIRED_FIELDS = {"id", "domain", "owner", "kpi", "threshold", "window", "action"}
OPTIONAL_FIELDS = ("description", "rollback")


def _load_parser():
    spec = importlib.util.find_spec("yaml")
    if spec is None:
        return json.loads
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None  # for mypy/static analyzers
    spec.loader.exec_module(module)  # type: ignore[assignment]
    return module.safe_load  # type: ignore[attr-defined]


_parse_config = _load_parser()


def _coerce_watcher(domain: str, watcher: Dict[str, Any]) -> Dict[str, Any]:
    if not isinstance(watcher, dict):
        raise ValueError("Watcher entries must be objects")
    name = watcher.get("name") or watcher.get("id")
    if not name:
        raise ValueError(f"Watcher entry missing identifier: {watcher}")
    record: Dict[str, Any] = {"id": name, "domain": domain}
    for key in ("owner", "kpi", "threshold", "window", "action"):
        if key not in watcher:
            raise ValueError(f"Watcher '{name}' missing required field '{key}'")
        record[key] = watcher[key]
    for field in OPTIONAL_FIELDS:
        if field in watcher:
            record[field] = watcher[field]
    return record


def _load_watchers_from_file(path: pathlib.Path) -> List[Dict[str, Any]]:
    data = _parse_config(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"Watcher configuration must be a mapping: {path}")
    domain = data.get("domain")
    if not isinstance(domain, str) or not domain.strip():
        raise ValueError(f"Watcher configuration missing 'domain': {path}")
    watchers = data.get("watchers")
    if not isinstance(watchers, list):
        raise ValueError(f"Watcher configuration must contain a 'watchers' list: {path}")
    return [_coerce_watcher(domain.strip(), watcher) for watcher in watchers]


def _load_watchers(path: pathlib.Path) -> List[Dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(f"Watcher configuration not found: {path}")
    if path.is_dir():
        entries: List[Dict[str, Any]] = []
        for candidate in sorted(path.iterdir()):
            if candidate.is_file() and candidate.suffix.lower() == ".yml":
                entries.extend(_load_watchers_from_file(candidate))
        if not entries:
            raise ValueError(f"No watcher definitions found in directory: {path}")
        return entries
    if path.suffix in {".yml", ".yaml", ".json"}:
        return _load_watchers_from_file(path)
    raise ValueError(f"Unsupported watcher configuration path: {path}")


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
    result = {
        "id": normalized["id"],
        "domain": normalized["domain"],
        "owner": normalized["owner"],
        "kpi": normalized["kpi"],
        "threshold": normalized["threshold"],
        "window": normalized["window"],
        "action": normalized["action"],
        "hash": digest,
    }
    for field in OPTIONAL_FIELDS:
        if field in watcher and watcher[field] is not None:
            result[field] = _normalize_field(watcher[field])
    result["description"] = watcher.get("description", "")
    return result


def generate_report(config: pathlib.Path) -> Dict[str, Any]:
    watchers = _load_watchers(config)
    validated = [_validate_watcher(w) for w in watchers]
    validated.sort(key=lambda entry: (entry["domain"], entry["id"]))
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
