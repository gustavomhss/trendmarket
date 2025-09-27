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
from typing import Any, Dict, List, Mapping, MutableMapping

CORE_FILENAME = "core.yaml"

REQUIRED_FIELDS = {"id", "domain", "domains", "owner", "description", "hook_id"}
OPTIONAL_FIELDS = ("kpi", "threshold", "window", "action", "rollback")

# cache interno para hooks
_HOOK_METADATA_CACHE: dict[str, dict[str, Any]] | None = None


def _load_parser():
    spec = importlib.util.find_spec("yaml")
    if spec is None:
        return json.loads
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)  # type: ignore
    return module.safe_load  # type: ignore


_parse_config = _load_parser()


def _load_hook_metadata() -> Dict[str, Dict[str, Any]]:
    """Carrega metadados de hooks para complementar watchers."""
    global _HOOK_METADATA_CACHE
    if _HOOK_METADATA_CACHE is not None:
        return _HOOK_METADATA_CACHE

    hooks_path = pathlib.Path("ops/hooks/a110.yml")
    if not hooks_path.exists():
        _HOOK_METADATA_CACHE = {}
        return _HOOK_METADATA_CACHE

    try:
        payload = _parse_config(hooks_path.read_text(encoding="utf-8"))
    except Exception:
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


def _normalize_string(
    value: Any, *, field: str, source: pathlib.Path, allow_empty: bool = False
) -> str:
    if not isinstance(value, str):
        raise ValueError(f"{source}: {field} must be a string")
    text = value.strip()
    if not text and not allow_empty:
        raise ValueError(f"{source}: {field} must not be empty")
    return text


def _extract_domains(payload: Mapping[str, Any], source: pathlib.Path) -> Dict[str, List[str]]:
    domains = payload.get("domains")
    if not isinstance(domains, Mapping) or not domains:
        raise ValueError(f"{source}: 'domains' must be a non-empty mapping")

    watcher_domains: Dict[str, List[str]] = {}
    for domain_raw, watchers in domains.items():
        domain = _normalize_string(domain_raw, field="domain name", source=source)
        if not isinstance(watchers, list) or not watchers:
            raise ValueError(f"{source}: domain '{domain}' must declare at least one watcher")

        seen: set[str] = set()
        for index, watcher_raw in enumerate(watchers, start=1):
            watcher_id = _normalize_string(
                watcher_raw,
                field=f"watcher #{index} in domain '{domain}'",
                source=source,
            )
            if watcher_id in seen:
                raise ValueError(
                    f"{source}: duplicated watcher '{watcher_id}' declared for domain '{domain}'"
                )
            seen.add(watcher_id)
            mapping = watcher_domains.setdefault(watcher_id, [])
            if domain not in mapping:
                mapping.append(domain)

    return watcher_domains


def _extract_watcher_metadata(payload: Mapping[str, Any], source: pathlib.Path) -> Dict[str, Dict[str, Any]]:
    watchers = payload.get("watchers")
    if not isinstance(watchers, Mapping) or not watchers:
        raise ValueError(f"{source}: 'watchers' must be a non-empty mapping")

    metadata: Dict[str, Dict[str, Any]] = {}
    for watcher_id_raw, watcher_payload in watchers.items():
        watcher_id = _normalize_string(watcher_id_raw, field="watcher id", source=source)
        if not isinstance(watcher_payload, MutableMapping):
            raise ValueError(f"{source}: watcher '{watcher_id}' must be defined as a mapping")

        owner_raw = watcher_payload.get("owner")
        if owner_raw is None:
            raise ValueError(f"{source}: watcher '{watcher_id}' missing required field 'owner'")
        owner = _normalize_string(owner_raw, field=f"watcher '{watcher_id}' owner", source=source)

        description_raw = watcher_payload.get("description", "")
        description = _normalize_string(
            description_raw if description_raw is not None else "",
            field=f"watcher '{watcher_id}' description",
            source=source,
            allow_empty=True,
        )

        entry: Dict[str, Any] = {"owner": owner, "description": description}
        for optional in OPTIONAL_FIELDS:
            if optional in watcher_payload and watcher_payload[optional] is not None:
                entry[optional] = _normalize_string(
                    watcher_payload[optional],
                    field=f"watcher '{watcher_id}' {optional}",
                    source=source,
                )

        metadata[watcher_id] = entry

    return metadata


def _load_watchers(path: pathlib.Path) -> List[Dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(f"Watcher configuration not found: {path}")
    if path.is_dir():
        raise ValueError(f"{path}: expected a consolidated watcher configuration file")
    data = _parse_config(path.read_text(encoding="utf-8"))
    if not isinstance(data, Mapping):
        raise ValueError(f"{path}: watcher configuration must be a mapping")

    watcher_domains = _extract_domains(data, path)
    watcher_metadata = _extract_watcher_metadata(data, path)

    undefined = sorted(set(watcher_domains) - set(watcher_metadata))
    if undefined:
        names = ", ".join(undefined)
        raise ValueError(f"{path}: undefined watcher metadata for: {names}")

    unassigned = sorted(set(watcher_metadata) - set(watcher_domains))
    if unassigned:
        names = ", ".join(unassigned)
        raise ValueError(f"{path}: watchers missing domain assignment: {names}")

    records: List[Dict[str, Any]] = []
    for watcher_id in sorted(watcher_metadata):
        domains = watcher_domains[watcher_id]
        record = {
            "id": watcher_id,
            "domains": domains,
            "domain": domains[0],
            **watcher_metadata[watcher_id],
        }
        records.append(record)

    return records


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
    if isinstance(value, list):
        return [_normalize_field(item) for item in value]
    return value


def _validate_watcher(watcher: Dict[str, Any]) -> Dict[str, Any]:
    missing = REQUIRED_FIELDS - watcher.keys()
    if missing:
        raise ValueError(f"Watcher '{watcher.get('id')}' missing fields: {sorted(missing)}")
    normalized = {key: _normalize_field(watcher[key]) for key in sorted(watcher.keys())}

    if not isinstance(normalized.get("domains"), list) or not normalized["domains"]:
        raise ValueError(f"Watcher '{watcher.get('id')}' must declare at least one domain assignment")
    if normalized.get("domain") != normalized["domains"][0]:
        raise ValueError(
            f"Watcher '{watcher.get('id')}' primary domain must match the first entry in 'domains'"
        )

    digest_payload = {
        key: normalized[key]
        for key in ("id", "owner", "domains", "description")
    }
    for field in OPTIONAL_FIELDS:
        if field in normalized:
            digest_payload[field] = normalized[field]

    encoded = json.dumps(digest_payload, sort_keys=True, ensure_ascii=False).encode("utf-8")
    digest = hashlib.sha256(encoded).hexdigest()
    normalized["hash"] = digest
    return normalized


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