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

REQUIRED_FIELDS = {"id", "domain", "owner", "kpi", "hook"}
OPTIONAL_FIELDS = ("description", "domains")


def _load_parser():
    spec = importlib.util.find_spec("yaml")
    if spec is None:
        return json.loads
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None  # for mypy/static analyzers
    spec.loader.exec_module(module)  # type: ignore[assignment]
    return module.safe_load  # type: ignore[attr-defined]


_parse_config = _load_parser()


def _coerce_core_watcher(
    primary_domain: str, all_domains: List[str], watcher_id: str, metadata: Dict[str, Any]
) -> Dict[str, Any]:
    if not isinstance(metadata, dict):
        raise ValueError(
            f"Watcher metadata for '{watcher_id}' in domain '{primary_domain}' must be a mapping"
        )
    record: Dict[str, Any] = {
        "id": watcher_id,
        "domain": primary_domain,
        "domains": all_domains,
    }
    for key in ("owner", "kpi", "hook"):
        if key not in metadata:
            raise ValueError(
                f"Watcher '{watcher_id}' missing required metadata field '{key}'"
            )
        record[key] = metadata[key]
    for field in OPTIONAL_FIELDS:
        if field in metadata:
            record[field] = metadata[field]
    return record


def _load_watchers_from_core(path: pathlib.Path, data: Dict[str, Any]) -> List[Dict[str, Any]]:
    domains = data.get("domains")
    catalog = data.get("watchers")
    if not isinstance(domains, dict) or not isinstance(catalog, dict):
        raise ValueError(
            f"Core watcher configuration must contain 'domains' and 'watchers' mappings: {path}"
        )

    domain_index: Dict[str, List[str]] = {}
    for domain, watchers in sorted(domains.items()):
        if not isinstance(domain, str) or not domain.strip():
            raise ValueError(
                f"Watcher domain names must be non-empty strings in {path}: {domain}"
            )
        if not isinstance(watchers, list):
            raise ValueError(
                f"Watcher list for domain '{domain}' must be a list: {path}"
            )
        for watcher_id in watchers:
            if not isinstance(watcher_id, str) or not watcher_id.strip():
                raise ValueError(
                    f"Watcher identifiers must be non-empty strings: {watcher_id}"
                )
            key = watcher_id.strip()
            domain_index.setdefault(key, [])
            domain_index[key].append(domain.strip())

    entries: List[Dict[str, Any]] = []
    for watcher_id, metadata in sorted(catalog.items()):
        if not isinstance(watcher_id, str) or not watcher_id.strip():
            raise ValueError(
                f"Watcher identifiers in catalog must be non-empty strings: {watcher_id}"
            )
        key = watcher_id.strip()
        domains_for_watcher = domain_index.get(key)
        if not domains_for_watcher:
            raise ValueError(
                f"Watcher '{key}' defined in catalog but not assigned to any domain"
            )
        unique_domains = sorted(dict.fromkeys(domains_for_watcher))
        entries.append(
            _coerce_core_watcher(unique_domains[0], unique_domains, key, metadata)
        )
    if len(entries) != len(catalog):
        raise ValueError(
            "Mismatch between watcher catalog entries and resolved watchers"
        )
    if not entries:
        raise ValueError(f"No watchers resolved from configuration: {path}")
    return entries


def _load_watchers_from_file(path: pathlib.Path) -> List[Dict[str, Any]]:
    data = _parse_config(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"Watcher configuration must be a mapping: {path}")

    if "domains" in data and "watchers" in data:
        return _load_watchers_from_core(path, data)

    raise ValueError(
        f"Unsupported watcher configuration format (expected core.yaml style): {path}"
    )


def _load_watchers(path: pathlib.Path) -> List[Dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(f"Watcher configuration not found: {path}")
    if path.is_dir():
        core_candidate = path / "core.yaml"
        if core_candidate.exists():
            return _load_watchers_from_file(core_candidate)
        raise ValueError(
            f"Watcher configuration directory must contain 'core.yaml': {path}"
        )
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
        "hook": normalized["hook"],
        "hash": digest,
    }
    for field in OPTIONAL_FIELDS:
        if field in watcher and watcher[field] is not None:
            result[field] = _normalize_field(watcher[field])
    result["description"] = watcher.get("description", "")
    return result


def _resolve_source_path(config: pathlib.Path) -> str:
    resolved = config.resolve()
    repo_root = pathlib.Path(__file__).resolve().parents[1]
    try:
        return str(resolved.relative_to(repo_root))
    except ValueError:
        return str(resolved)


def generate_report(config: pathlib.Path) -> Dict[str, Any]:
    config_path = config.resolve()
    watchers = _load_watchers(config_path)
    validated = [_validate_watcher(w) for w in watchers]
    validated.sort(key=lambda entry: (entry["domain"], entry["id"]))
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source": _resolve_source_path(config_path),
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
