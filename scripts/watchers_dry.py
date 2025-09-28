#!/usr/bin/env python3
"""Dry-run validator for watcher coverage across domains."""
from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any, Callable, Dict, Iterable, List, Set, Tuple

EXPECTED_DOMAIN_WATCHERS: Dict[str, Set[str]] = {
    "DEC": {
        "metrics_decision_hook_gap_watch",
        "model_drift_watch",
        "slo_budget_breach_watch",
    },
    "PM": {
        "oracle_divergence_watch",
        "fx_delta_benchmark_watch",
        "auction_invariant_breach_watch",
        "slo_budget_breach_watch",
    },
    "ML": {
        "model_drift_watch",
        "ab_srm_watch",
        "runtime_eol_watch",
        "image_vuln_regression_watch",
    },
    "DATA": {
        "cdc_lag_watch",
        "schema_registry_drift_watch",
        "dbt_test_failure_watch",
        "doc_coverage_watch",
        "data_contract_break_watch",
    },
    "PLAT": {
        "tracing_sampling_watch",
        "alert_storm_watch",
        "slo_budget_breach_watch",
        "policy_violation_watch",
        "okr_risk_alignment_watch",
    },
    "FE": {
        "web_cwv_regression_watch",
        "api_breaking_change_watch",
    },
    "SEC/PRIV": {
        "dep_vuln_watch",
        "image_vuln_regression_watch",
        "idp_keys_rotation_watch",
        "dp_budget_breach_watch",
        "formal_verification_gate_watch",
    },
    "INT": {
        "api_breaking_change_watch",
        "cache_ttl_misuse_watch",
        "cls_payin_cutoff_watch",
    },
}

MANDATORY_GLOBAL_WATCHERS: Set[str] = {
    "api_breaking_change_watch",
    "schema_registry_drift_watch",
    "data_contract_break_watch",
    "dbt_test_failure_watch",
    "cdc_lag_watch",
    "slo_budget_breach_watch",
    "model_drift_watch",
    "metrics_decision_hook_gap_watch",
    "formal_verification_gate_watch",
    "web_cwv_regression_watch",
    "okr_risk_alignment_watch",
    "dp_budget_breach_watch",
    "runtime_eol_watch",
    "dep_vuln_watch",
    "oracle_divergence_watch",
    "fx_delta_benchmark_watch",
}

REQUIRED_FIELDS = {
    "name",
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


def _load_parser() -> Callable[[str], Any]:
    spec = importlib.util.find_spec("yaml")
    if spec is None:
        return json.loads

    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)  # type: ignore
    return module.safe_load  # type: ignore[return-value]


_parse_payload = _load_parser()


def _load_watchers_file(path: Path) -> Tuple[Dict[str, Set[str]], bool]:
    try:
        payload = _parse_payload(path.read_text(encoding="utf-8"))
    except Exception as exc:  # pragma: no cover - defensive
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

        return results, True

    domain_raw = payload.get("domain")
    domain = str(domain_raw).upper() if domain_raw else path.stem.upper()
    watchers = payload.get("watchers")

    if not isinstance(watchers, list) or not watchers:
        raise WatcherValidationError(f"{path}: watchers must be a non-empty list")

    seen: Set[str] = set()
    for idx, watcher in enumerate(watchers):
        if not isinstance(watcher, dict):
            raise WatcherValidationError(
                f"{path}: watcher #{idx + 1} must be a mapping with required fields"
            )

        missing = REQUIRED_FIELDS - watcher.keys()
        if missing:
            raise WatcherValidationError(
                f"{path}: watcher '{watcher.get('name', '<unknown>')}' missing fields: {sorted(missing)}"
            )

        name = str(watcher.get("name")).strip()
        if not name:
            raise WatcherValidationError(f"{path}: watcher #{idx + 1} missing a valid name")

        rollback_value = str(watcher.get("rollback")).lower()
        if rollback_value not in VALID_ROLLBACK:
            raise WatcherValidationError(
                f"{path}: watcher '{name}' has invalid rollback value '{watcher.get('rollback')}'"
            )

        if name in seen:
            raise WatcherValidationError(
                f"{path}: duplicated watcher entry '{name}' for domain '{domain}'"
            )

        seen.add(name)

    return {domain: seen}, False


def _summarize_domain_counts(domain_watchers: Dict[str, Set[str]]) -> Iterable[str]:
    for domain in sorted(domain_watchers):
        yield f"  - {domain}: {len(domain_watchers[domain])} watcher(s)"


def _load_watchers() -> Tuple[Dict[str, Set[str]], List[str]]:
    watchers_dir = Path("ops/watchers")
    errors: List[str] = []
    domain_watchers: Dict[str, Set[str]] = {}
    aggregated_domains: Set[str] = set()
    non_aggregated_domains: Set[str] = set()

    if not watchers_dir.is_dir():
        errors.append("ops/watchers directory not found")
        return domain_watchers, errors

    all_paths = sorted({*watchers_dir.glob("*.yml"), *watchers_dir.glob("*.yaml")})

    for path in all_paths:
        try:
            domain_entries, is_aggregated = _load_watchers_file(path)
        except WatcherValidationError as exc:
            errors.append(str(exc))
            continue

        if is_aggregated:
            aggregated_domains.update(domain_entries.keys())

        for domain, watchers in domain_entries.items():
            existing = domain_watchers.get(domain)

            if existing is None:
                domain_watchers[domain] = set(watchers)
                if not is_aggregated:
                    non_aggregated_domains.add(domain)
                continue

            if is_aggregated:
                existing.update(watchers)
                aggregated_domains.add(domain)
                continue

            if domain in aggregated_domains:
                if domain in non_aggregated_domains:
                    errors.append(
                        f"duplicate watcher definition for domain '{domain}' in {path}"
                    )
                    continue

                existing.update(watchers)
                non_aggregated_domains.add(domain)
                continue

            errors.append(f"duplicate watcher definition for domain '{domain}' in {path}")

    return domain_watchers, errors


def main() -> int:
    domain_watchers, errors = _load_watchers()

    missing_directory = any(
        message == "ops/watchers directory not found" for message in errors
    )

    if not missing_directory:
        expected_domains = set(EXPECTED_DOMAIN_WATCHERS)

        missing_domains = expected_domains - domain_watchers.keys()
        if missing_domains:
            errors.append(
                "missing watcher definitions for domain(s): "
                + ", ".join(sorted(missing_domains))
            )

        extra_domains = domain_watchers.keys() - expected_domains
        if extra_domains:
            errors.append(
                "unexpected watcher domains found: " + ", ".join(sorted(extra_domains))
            )

        for domain, expected_watchers in EXPECTED_DOMAIN_WATCHERS.items():
            actual = domain_watchers.get(domain, set())
            missing = expected_watchers - actual
            if missing:
                errors.append(
                    f"domain '{domain}' missing required watcher(s): "
                    + ", ".join(sorted(missing))
                )

        union_watchers: Set[str] = set()
        for watchers in domain_watchers.values():
            union_watchers.update(watchers)

        missing_globals = MANDATORY_GLOBAL_WATCHERS - union_watchers
        if missing_globals:
            errors.append(
                "mandatory watcher(s) absent from configuration: "
                + ", ".join(sorted(missing_globals))
            )
    else:
        union_watchers = set()

    if errors:
        print("[watchers.dry] validation failed:", file=sys.stderr)
        for message in errors:
            print(f"  - {message}", file=sys.stderr)
        return 1

    print("[watchers.dry] watcher coverage OK:")
    for line in _summarize_domain_counts(domain_watchers):
        print(line)
    print(f"  - total unique watchers: {len(union_watchers)}")
    print("[watchers.dry] all mandatory watchers are configured.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
