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

CORE_FILENAME = "core.yaml"

REQUIRED_FIELDS = {"id", "domain", "owner", "kpi", "threshold", "window", "action", "hook_id"}
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

    # Allow both per-domain configs (with a single domain) and aggregated JSON
    # inventories where each watcher already declares its domain.
    domain = data.get("domain")
    watchers = data.get("watchers")
    if isinstance(domain, str) and domain.strip():
        if not isinstance(watchers, list):
            raise ValueError(
                f"Watcher configuration must contain a 'watchers' list: {path}"
            )
        return [_coerce_watcher(domain.strip(), watcher) for watcher in watchers]

    if isinstance(watchers, list):
        entries: List[Dict[str, Any]] = []
        for watcher in watchers:
            if not isinstance(watcher, dict):
                raise ValueError(
                    f"Aggregated watcher entries must be objects: {path}"
                )
            watcher_domain = watcher.get("domain")
            if not isinstance(watcher_domain, str) or not watcher_domain.strip():
                raise ValueError(
                    f"Aggregated watcher missing domain information: {watcher}"
                )
            entries.append(_coerce_watcher(watcher_domain.strip(), watcher))
        if entries:
            return entries

    raise ValueError(
        f"Watcher configuration missing 'domain' or 'watchers' list: {path}"
    )


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


def _resolve_core_path(config: pathlib.Path) -> pathlib.Path:
    if config.is_dir():
        return config / CORE_FILENAME
    if config.name == CORE_FILENAME:
        return config
    return config.parent / CORE_FILENAME


def _load_core_bindings(core_path: pathlib.Path) -> Dict[tuple[str, str], str]:
    if not core_path.exists():
        raise FileNotFoundError(f"Watcher core configuration not found: {core_path}")

    data = _parse_config(core_path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"Watcher core configuration must be a mapping: {core_path}")

    watchers_section = data.get("watchers")
    if not isinstance(watchers_section, dict) or not watchers_section:
        raise ValueError(f"Watcher core configuration missing watcher definitions: {core_path}")

    bindings: Dict[tuple[str, str], str] = {}
    for watcher_id, meta in watchers_section.items():
        hooks_map = meta.get("hooks")
        if not isinstance(hooks_map, dict) or not hooks_map:
            continue
        for domain_key, hook_value in hooks_map.items():
            if not isinstance(domain_key, str) or not isinstance(hook_value, str):
                continue
            domain = domain_key.strip().upper()
            hook = hook_value.strip()
            if not domain or not hook:
                continue
            bindings[(domain, watcher_id)] = hook

    if not bindings:
        raise ValueError(f"Watcher core configuration missing hook bindings: {core_path}")

    return bindings


def _attach_hooks(
    watchers: List[Dict[str, Any]],
    bindings: Dict[tuple[str, str], str],
) -> List[Dict[str, Any]]:
    enriched: List[Dict[str, Any]] = []
    for watcher in watchers:
        domain = str(watcher.get("domain", "")).strip().upper()
        name = watcher.get("id")
        if not domain or not name:
            raise ValueError(f"Watcher entry missing domain or id: {watcher}")
        hook = bindings.get((domain, name))
        if hook is None:
            raise ValueError(
                f"Watcher '{name}' in domain '{domain}' missing hook assignment in core.yaml"
            )
        extended = dict(watcher)
        extended["hook_id"] = hook
        enriched.append(extended)
    return enriched


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
        "hook_id": normalized["hook_id"],
        "hash": digest,
    }
    for field in OPTIONAL_FIELDS:
        if field in watcher and watcher[field] is not None:
            result[field] = _normalize_field(watcher[field])
    result["description"] = watcher.get("description", "")
    return result


def generate_report(config: pathlib.Path) -> Dict[str, Any]:
    watchers = _load_watchers(config)
    core_bindings = _load_core_bindings(_resolve_core_path(config))
    enriched = _attach_hooks(watchers, core_bindings)
    validated = [_validate_watcher(w) for w in enriched]
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
