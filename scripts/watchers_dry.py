#!/usr/bin/env python3
"""Dry-run validator for watcher coverage across domains."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Dict, Iterable, Set, Tuple

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


def _load_watchers_file(path: Path) -> Tuple[str, Set[str]]:
    try:
        payload = json.loads(path.read_text())
    except json.JSONDecodeError as exc:  # pragma: no cover - defensive
        raise WatcherValidationError(f"{path}: invalid JSON/YAML payload: {exc}")

    if not isinstance(payload, dict):
        raise WatcherValidationError(f"{path}: expected a mapping with domain and watchers")

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

    return domain, seen


def _summarize_domain_counts(domain_watchers: Dict[str, Set[str]]) -> Iterable[str]:
    for domain in sorted(domain_watchers):
        yield f"  - {domain}: {len(domain_watchers[domain])} watcher(s)"


def main() -> int:
    watchers_dir = Path("ops/watchers")
    if not watchers_dir.is_dir():
        print("[watchers.dry] ops/watchers directory not found", file=sys.stderr)
        return 1

    errors = []
    domain_watchers: Dict[str, Set[str]] = {}

    for path in sorted(watchers_dir.glob("*.yml")):
        try:
            domain, watchers = _load_watchers_file(path)
        except WatcherValidationError as exc:
            errors.append(str(exc))
            continue

        if domain in domain_watchers:
            errors.append(f"duplicate watcher definition for domain '{domain}' in {path}")
            continue

        domain_watchers[domain] = watchers

    expected_domains = set(EXPECTED_DOMAIN_WATCHERS)

    missing_domains = expected_domains - domain_watchers.keys()
    if missing_domains:
        errors.append(
            "missing watcher definitions for domain(s): " + ", ".join(sorted(missing_domains))
        )

    extra_domains = domain_watchers.keys() - expected_domains
    if extra_domains:
        errors.append("unexpected watcher domains found: " + ", ".join(sorted(extra_domains)))

    for domain, expected_watchers in EXPECTED_DOMAIN_WATCHERS.items():
        actual = domain_watchers.get(domain, set())
        missing = expected_watchers - actual
        if missing:
            errors.append(
                f"domain '{domain}' missing required watcher(s): " + ", ".join(sorted(missing))
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
