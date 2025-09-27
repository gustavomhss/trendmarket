#!/usr/bin/env python3
"""Dry-run validator for A110 hook coverage."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Dict, Iterable, List, Set, Tuple

REQUIRED_HOOK_FIELDS = {
    "hook",
    "domain",
    "watchers",
    "kpi",
    "threshold",
    "window",
    "action",
    "owner",
    "rollback",
}
VALID_ROLLBACK = {"yes", "no"}


class WatcherValidationError(RuntimeError):
    """Represents a validation error during watcher loading."""


def _load_watchers_file(path: Path) -> Dict[str, Set[str]]:
    try:
        payload = json.loads(path.read_text())
    except json.JSONDecodeError as exc:  # pragma: no cover - defensive
        raise WatcherValidationError(f"{path}: invalid JSON/YAML payload: {exc}")

    if not isinstance(payload, dict):
        raise WatcherValidationError(f"{path}: expected a mapping with domain and watchers")

    if "domains" in payload:
        domains = payload.get("domains")
        if not isinstance(domains, dict) or not domains:
            raise WatcherValidationError(f"{path}: domains must be a non-empty mapping")

        results: Dict[str, Set[str]] = {}
        for domain_raw, watchers in domains.items():
            domain = str(domain_raw or "").strip().upper()
            if not domain:
                raise WatcherValidationError(f"{path}: domain entries must have a valid name")
            if not isinstance(watchers, list) or not watchers:
                raise WatcherValidationError(
                    f"{path}: domain '{domain}' must map to a non-empty watcher list"
                )

            names: Set[str] = set()
            for idx, watcher in enumerate(watchers):
                if not isinstance(watcher, str):
                    raise WatcherValidationError(
                        f"{path}: watcher #{idx + 1} for domain '{domain}' must be a string"
                    )
                name = watcher.strip()
                if not name:
                    raise WatcherValidationError(
                        f"{path}: watcher #{idx + 1} for domain '{domain}' missing a valid name"
                    )
                if name in names:
                    raise WatcherValidationError(
                        f"{path}: duplicated watcher entry '{name}' for domain '{domain}'"
                    )
                names.add(name)

            results[domain] = names

        return results

    domain_raw = payload.get("domain")
    domain = str(domain_raw).upper() if domain_raw else path.stem.upper()
    watchers = payload.get("watchers")

    if not isinstance(watchers, list) or not watchers:
        raise WatcherValidationError(f"{path}: watchers must be a non-empty list")

    names: Set[str] = set()
    for idx, watcher in enumerate(watchers):
        if not isinstance(watcher, dict):
            raise WatcherValidationError(
                f"{path}: watcher #{idx + 1} must be a mapping with required fields"
            )
        name = str(watcher.get("name", "")).strip()
        if not name:
            raise WatcherValidationError(f"{path}: watcher #{idx + 1} missing a valid name")
        if name in names:
            raise WatcherValidationError(
                f"{path}: duplicated watcher entry '{name}' for domain '{domain}'"
            )
        names.add(name)

    return {domain: names}


def _load_watchers() -> Tuple[Dict[str, Set[str]], List[str]]:
    watchers_dir = Path("ops/watchers")
    if not watchers_dir.is_dir():
        return {}, ["ops/watchers directory not found"]

    errors: List[str] = []
    domain_watchers: Dict[str, Set[str]] = {}
    provenance: Dict[str, Dict[str, Set[str]]] = {}
    yaml_sources: Dict[str, str] = {}
    yml_sources: Dict[str, str] = {}

    def _record(domain: str, watchers: Set[str], source: Path) -> None:
        source_key = str(source)
        domain_watchers.setdefault(domain, set()).update(watchers)
        provenance.setdefault(domain, {})[source_key] = set(watchers)

    yaml_paths = sorted({path for path in watchers_dir.glob("*.yaml")})

    for path in yaml_paths:
        try:
            domain_entries = _load_watchers_file(path)
        except WatcherValidationError as exc:
            errors.append(str(exc))
            continue

        for domain, watchers in domain_entries.items():
            if domain in yaml_sources:
                errors.append(
                    f"duplicate watcher definition for domain '{domain}' in {path}"
                )
                continue

            yaml_sources[domain] = str(path)
            _record(domain, watchers, path)

    yml_paths = sorted({path for path in watchers_dir.glob("*.yml")})

    for path in yml_paths:
        try:
            domain_entries = _load_watchers_file(path)
        except WatcherValidationError as exc:
            errors.append(str(exc))
            continue

        for domain, watchers in domain_entries.items():
            if domain in yml_sources:
                errors.append(
                    f"duplicate watcher definition for domain '{domain}' in {path}"
                )
                continue

            yml_sources[domain] = str(path)
            _record(domain, watchers, path)

    for domain, sources in provenance.items():
        if len(sources) <= 1:
            continue

        # Prefer the aggregated inventory ("*.yaml") as the baseline when present.
        sorted_sources = sorted(
            sources.items(),
            key=lambda item: (0 if item[0].endswith(".yaml") else 1, item[0]),
        )

        baseline_source, baseline_watchers = sorted_sources[0]
        for source, watchers in sorted_sources[1:]:
            missing_in_source = baseline_watchers - watchers
            missing_in_baseline = watchers - baseline_watchers
            if not missing_in_source and not missing_in_baseline:
                continue

            diff_parts: List[str] = []
            if missing_in_source:
                formatted = ", ".join(sorted(missing_in_source))
                diff_parts.append(
                    f"{source} missing [{formatted}] compared to {baseline_source}"
                )
            if missing_in_baseline:
                formatted = ", ".join(sorted(missing_in_baseline))
                diff_parts.append(
                    f"{baseline_source} missing [{formatted}] compared to {source}"
                )

            message = "; ".join(diff_parts)
            errors.append(
                f"watcher mismatch for domain '{domain}': {message}"
            )

    return domain_watchers, errors


def _summarize_hooks(domain_to_hook: Dict[str, Tuple[str, Set[str]]]) -> Iterable[str]:
    for domain in sorted(domain_to_hook):
        hook_name, watchers = domain_to_hook[domain]
        watchers_list = ", ".join(sorted(watchers))
        yield f"  - {domain} :: {hook_name} -> [{watchers_list}]"


def main() -> int:
    domain_watchers, watcher_errors = _load_watchers()
    if watcher_errors:
        print("[hooks.dry] unable to load watcher inventory:", file=sys.stderr)
        for message in watcher_errors:
            print(f"  - {message}", file=sys.stderr)
        return 1

    hooks_path = Path("ops/hooks/a110.yml")
    if not hooks_path.is_file():
        print("[hooks.dry] ops/hooks/a110.yml not found", file=sys.stderr)
        return 1

    try:
        payload = json.loads(hooks_path.read_text())
    except json.JSONDecodeError as exc:
        print(f"[hooks.dry] invalid JSON/YAML payload: {exc}", file=sys.stderr)
        return 1

    if not isinstance(payload, list) or not payload:
        print("[hooks.dry] expected a non-empty list of hook mappings", file=sys.stderr)
        return 1

    errors: List[str] = []
    domain_to_hook: Dict[str, Tuple[str, Set[str]]] = {}

    for idx, entry in enumerate(payload):
        if not isinstance(entry, dict):
            errors.append(f"entry #{idx + 1} must be a mapping")
            continue

        missing = REQUIRED_HOOK_FIELDS - entry.keys()
        if missing:
            errors.append(
                f"hook entry #{idx + 1} missing fields: {sorted(missing)}"
            )
            continue

        hook_name = str(entry.get("hook", "")).strip()
        domain = str(entry.get("domain", "")).upper()
        watchers = entry.get("watchers")

        if not hook_name:
            errors.append(f"hook entry #{idx + 1} is missing a valid hook name")
        if not domain:
            errors.append(f"hook entry '{hook_name or idx + 1}' missing a domain identifier")

        if not isinstance(watchers, list) or not watchers:
            errors.append(f"hook '{hook_name or domain}' must declare at least one watcher")
            continue

        watcher_names: Set[str] = set()
        for watcher in watchers:
            if not isinstance(watcher, str) or not watcher.strip():
                errors.append(
                    f"hook '{hook_name or domain}' has an invalid watcher reference: {watcher!r}"
                )
                continue
            watcher_names.add(watcher.strip())

        rollback_value = str(entry.get("rollback", "")).lower()
        if rollback_value not in VALID_ROLLBACK:
            errors.append(
                f"hook '{hook_name or domain}' has invalid rollback value '{entry.get('rollback')}'"
            )

        if domain in domain_to_hook:
            errors.append(
                f"duplicate hook definition for domain '{domain}' (hook '{hook_name}')"
            )
        else:
            domain_to_hook[domain] = (hook_name, watcher_names)

    watcher_domains = set(domain_watchers)
    hook_domains = set(domain_to_hook)

    missing_domains = watcher_domains - hook_domains
    if missing_domains:
        errors.append(
            "hooks missing for domain(s): " + ", ".join(sorted(missing_domains))
        )

    extra_domains = hook_domains - watcher_domains
    if extra_domains:
        errors.append(
            "hook(s) defined without watcher inventory: " + ", ".join(sorted(extra_domains))
        )

    for domain, watchers in domain_watchers.items():
        hook_entry = domain_to_hook.get(domain)
        if not hook_entry:
            continue
        hook_name, hook_watchers = hook_entry
        missing = watchers - hook_watchers
        if missing:
            errors.append(
                f"hook '{hook_name}' missing watcher(s) for domain '{domain}': "
                + ", ".join(sorted(missing))
            )
        extra = hook_watchers - watchers
        if extra:
            errors.append(
                f"hook '{hook_name}' references undefined watcher(s) for domain '{domain}': "
                + ", ".join(sorted(extra))
            )

    if errors:
        print("[hooks.dry] validation failed:", file=sys.stderr)
        for message in errors:
            print(f"  - {message}", file=sys.stderr)
        return 1

    print("[hooks.dry] hook coverage OK:")
    for line in _summarize_hooks(domain_to_hook):
        print(line)
    print("[hooks.dry] all watchers are mapped to Gate A110 hooks.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
