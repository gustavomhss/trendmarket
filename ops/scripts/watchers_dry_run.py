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
from typing import Any, Dict, List, Optional

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

_HOOK_METADATA_CACHE: Optional[Dict[str, Dict[str, Any]]] = None


def _load_hook_metadata() -> Dict[str, Dict[str, Any]]:
    global _HOOK_METADATA_CACHE
    if _HOOK_METADATA_CACHE is not None:
        return _HOOK_METADATA_CACHE

    hooks_path = pathlib.Path("ops/hooks/a110.yml")
    if not hooks_path.exists():
        _HOOK_METADATA_CACHE = {}
        return _HOOK_METADATA_CACHE

    try:
        payload = _parse_config(hooks_path.read_text(encoding="utf-8"))
    except Exception:  # pragma: no cover - defensive guard for malformed hooks files
        _HOOK_METADATA_CACHE = {}
        return _HOOK_METADATA_CACHE

    hooks = payload.get("hooks")
    if not isinstance(hooks, list):
        _HOOK_METADATA_CACHE = {}
        return _HOOK_METADATA_CACHE

    metadata: Dict[str, Dict[str, Any]] = {}
    for entry in hooks:
        if not isinstance(entry, dict):
            continue
        watchers = entry.get("watchers")
        if not isinstance(watchers, list):
            continue
        for watcher in watchers:
            if not isinstance(watcher, str):
                continue
            name = watcher.strip()
            if not name:
                continue
            metadata[name] = {
                "hook": entry.get("hook"),
                "owner": entry.get("owner"),
                "kpi": entry.get("kpi"),
                "threshold": entry.get("threshold"),
                "window": entry.get("window"),
                "action": entry.get("action"),
                "rollback": entry.get("rollback"),
            }

    _HOOK_METADATA_CACHE = metadata
    return metadata


def _coerce_aggregated_watcher(
    path: pathlib.Path,
    domain: str,
    watcher_name: str,
    watcher_details: Dict[str, Any],
) -> Dict[str, Any]:
    record: Dict[str, Any] = {"id": watcher_name, "domain": domain}

    hook_metadata = _load_hook_metadata().get(watcher_name, {})

    def _resolve(*candidates: Any) -> Any:
        for value in candidates:
            if value is None:
                continue
            if isinstance(value, str):
                trimmed = value.strip()
                if trimmed:
                    return trimmed
                continue
            return value
        return None

    owner = _resolve(watcher_details.get("owner"), hook_metadata.get("owner"))
    kpi = _resolve(watcher_details.get("kpi"), hook_metadata.get("kpi"))
    threshold = _resolve(watcher_details.get("threshold"), hook_metadata.get("threshold"))
    window = _resolve(watcher_details.get("window"), hook_metadata.get("window"))
    action = _resolve(
        watcher_details.get("action"),
        hook_metadata.get("action"),
        watcher_details.get("hook"),
    )

    missing = [
        field
        for field, value in (
            ("owner", owner),
            ("kpi", kpi),
            ("threshold", threshold),
            ("window", window),
            ("action", action),
        )
        if value is None or (isinstance(value, str) and not value)
    ]
    if missing:
        raise ValueError(
            "Watcher '%s' for domain '%s' missing required fields %s in %s"
            % (watcher_name, domain, missing, path)
        )

    record.update(
        {
            "owner": owner,
            "kpi": kpi,
            "threshold": threshold,
            "window": window,
            "action": action,
        }
    )

    if "rollback" in watcher_details:
        record["rollback"] = watcher_details["rollback"]
    elif "rollback" in hook_metadata:
        record["rollback"] = hook_metadata["rollback"]

    if "description" in watcher_details:
        record["description"] = watcher_details["description"]

    return record


def _load_watchers_from_aggregated(
    path: pathlib.Path, payload: Dict[str, Any]
) -> List[Dict[str, Any]]:
    domains = payload.get("domains")
    if not isinstance(domains, dict) or not domains:
        raise ValueError(f"Aggregated watcher config missing 'domains': {path}")

    watchers_map = payload.get("watchers")
    if not isinstance(watchers_map, dict) or not watchers_map:
        raise ValueError(f"Aggregated watcher config missing 'watchers': {path}")

    entries: List[Dict[str, Any]] = []
    for domain_key, watchers in domains.items():
        domain = str(domain_key or "").strip()
        if not domain:
            raise ValueError(f"Invalid domain entry in aggregated config: {path}")
        if not isinstance(watchers, list) or not watchers:
            raise ValueError(
                f"Domain '{domain}' must map to a non-empty list of watchers in {path}"
            )
        for idx, watcher in enumerate(watchers, start=1):
            if not isinstance(watcher, str):
                raise ValueError(
                    f"Watcher #{idx} for domain '{domain}' must be a string in {path}"
                )
            watcher_name = watcher.strip()
            if not watcher_name:
                raise ValueError(
                    f"Watcher #{idx} for domain '{domain}' missing name in {path}"
                )
            details = watchers_map.get(watcher_name)
            if not isinstance(details, dict):
                raise ValueError(
                    f"Watcher '{watcher_name}' referenced by domain '{domain}' missing definition in {path}"
                )
            entries.append(
                _coerce_aggregated_watcher(path, domain, watcher_name, details)
            )

    return entries


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
    if "domains" in data and "watchers" in data:
        return _load_watchers_from_aggregated(path, data)
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
