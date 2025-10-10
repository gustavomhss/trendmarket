#!/usr/bin/env python3
"""Quality checks for OBS-3 Prometheus evidence (Thread 5)."""

from __future__ import annotations

import argparse
import json
import math
import os
from pathlib import Path
import sys
from typing import Dict, Iterable, List, Mapping, MutableMapping, Optional, Sequence, Tuple

import requests


CONNECT_TIMEOUT = 3.0
DEFAULT_READ_TIMEOUT = 5.0
CLOSURE_THRESHOLD = 3.0
LOW_TRAFFIC_EPS = 1e-9


class QualityCheckError(Exception):
    """Base exception for quality check failures."""


class DatasetMissingError(QualityCheckError):
    """Raised when required datasets are missing."""


class CheckFailedError(QualityCheckError):
    """Raised when a validation fails."""


class HttpQueryError(QualityCheckError):
    """Raised for HTTP/transport errors."""


class JsonStructureError(QualityCheckError):
    """Raised when JSON payloads are malformed."""


class NumericHealthError(QualityCheckError):
    """Raised when NaN/Inf values are detected."""


class TelemetryUnavailableError(QualityCheckError):
    """Raised when Prometheus telemetry is fully unavailable."""


class PrometheusResult:
    """Container for parsed Prometheus results."""

    def __init__(self, result_type: str, result: Sequence[Mapping[str, object]]):
        self.result_type = result_type
        self.result = list(result)

    def __bool__(self) -> bool:
        return bool(self.result)


def log(message: str, *, error: bool = False, verbose: bool = True) -> None:
    """Emit log messages to stdout or stderr."""

    if not verbose and not error:
        return
    stream = sys.stderr if error else sys.stdout
    stream.write(f"{message}\n")


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run OBS-3 Prometheus quality checks (Thread 5)."
    )
    parser.add_argument(
        "--evidence-dir",
        default="out/obs_gatecheck/evidence",
        help="Directory containing collected Prometheus evidence (default: %(default)s).",
    )
    parser.add_argument(
        "--manifest",
        help="Target manifest file (default: <evidence-dir>/prom_scrape.json).",
    )
    parser.add_argument(
        "--addr",
        default=":9090",
        help="Prometheus host:port for live queries (default: %(default)s).",
    )
    parser.add_argument(
        "--live",
        action="store_true",
        help="Query Prometheus live instead of using local evidence files.",
    )
    parser.add_argument(
        "--window",
        default="5m",
        help="Range window for rate/increase operations (default: %(default)s).",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Fail if any partial check cannot be completed due to missing data.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Do not write the manifest; print JSON to stdout instead.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Emit additional informational logs.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=DEFAULT_READ_TIMEOUT,
        help="Read timeout in seconds for HTTP requests (default: %(default).1f).",
    )
    return parser.parse_args(argv)


def is_finite(value: float) -> bool:
    return math.isfinite(value)


def parse_le(le_value: str) -> float:
    if le_value in {"+Inf", "inf", "Inf", "+infinity", "Infinity"}:
        return math.inf
    try:
        return float(le_value)
    except ValueError as exc:  # pragma: no cover - protective guard
        raise JsonStructureError(f"Invalid histogram boundary: {le_value}") from exc


def sanitize_query_name(name: str) -> str:
    sanitized = "".join(c if c.isalnum() else "_" for c in name.lower())
    while "__" in sanitized:
        sanitized = sanitized.replace("__", "_")
    return sanitized.strip("_")


class EvidenceStore:
    """Loads Prometheus evidence JSON files for offline evaluation."""

    OFFLINE_ALIAS_MAP: Mapping[str, Sequence[str]] = {
        "up": ("prom_up.json",),
        "p75": ("prom_p75_rec.json", "prom_p75_adhoc.json"),
        "p95": ("prom_p95_rec.json", "prom_p95_adhoc.json"),
        "hist_increase": (
            "prom_histogram_increase.json",
            "prom_bucket_increase.json",
            "prom_latency_bucket_increase.json",
        ),
        "bucket_rate": (
            "prom_histogram_bucket_rate.json",
            "prom_bucket_rate.json",
        ),
        "count_rate": (
            "prom_histogram_count_rate.json",
            "prom_count_rate.json",
        ),
        "avg_latency": (
            "prom_latency_avg.json",
            "prom_histogram_avg.json",
        ),
        "hook_rate": (
            "prom_hook_rate.json",
            "prom_hook_exec_rate.json",
        ),
        "telemetry_duration": (
            "prom_scrape_duration.json",
            "prom_scrape_p95.json",
        ),
        "telemetry_samples": (
            "prom_scrape_samples.json",
        ),
        "telemetry_interval": (
            "prom_target_interval.json",
            "prom_scrape_interval.json",
        ),
    }

    def __init__(self, evidence_dir: Path, verbose: bool = False) -> None:
        self.evidence_dir = evidence_dir
        self.verbose = verbose
        self.payloads: Dict[str, object] = {}
        if evidence_dir.exists():
            for path in sorted(evidence_dir.glob("prom_*.json")):
                try:
                    with path.open("r", encoding="utf-8") as handle:
                        data = json.load(handle)
                    self.payloads[path.name] = data
                except json.JSONDecodeError as exc:
                    raise JsonStructureError(
                        f"Invalid JSON structure in evidence file: {path}"
                    ) from exc
        else:
            self.payloads = {}

    def _match_alias_files(self, alias: str) -> List[Path]:
        names = list(self.OFFLINE_ALIAS_MAP.get(alias, ()))
        return [self.evidence_dir / name for name in names if (self.evidence_dir / name).exists()]

    def _search_by_query(self, expected: str) -> Optional[Tuple[str, object]]:
        for name, payload in self.payloads.items():
            query = None
            if isinstance(payload, Mapping):
                query = (
                    payload.get("query")
                    or payload.get("expression")
                    or payload.get("expr")
                )
                if query and isinstance(query, str) and query.strip() == expected.strip():
                    return name, payload
            if isinstance(payload, Mapping):
                meta = payload.get("metadata")
                if isinstance(meta, Mapping):
                    query = meta.get("query") or meta.get("expression")
                    if query and isinstance(query, str) and query.strip() == expected.strip():
                        return name, payload
        return None

    def load_query(self, query: str, alias: Optional[str] = None) -> PrometheusResult:
        candidate_payload: Optional[object] = None
        candidate_name: Optional[str] = None
        alias_files: List[Path] = []
        if alias:
            alias_files = self._match_alias_files(alias)
        if alias_files:
            for path in alias_files:
                try:
                    with path.open("r", encoding="utf-8") as handle:
                        candidate_payload = json.load(handle)
                        candidate_name = path.name
                        break
                except json.JSONDecodeError as exc:
                    raise JsonStructureError(
                        f"Invalid JSON structure in evidence file: {path}"
                    ) from exc
        if candidate_payload is None:
            by_query = self._search_by_query(query)
            if by_query:
                candidate_name, candidate_payload = by_query
        if candidate_payload is None and alias:
            sanitized = sanitize_query_name(alias)
            candidate_path = self.evidence_dir / f"{sanitized}.json"
            if candidate_path.exists():
                with candidate_path.open("r", encoding="utf-8") as handle:
                    candidate_payload = json.load(handle)
                    candidate_name = candidate_path.name
        if candidate_payload is None:
            sanitized = sanitize_query_name(query)
            candidate_path = self.evidence_dir / f"{sanitized}.json"
            if candidate_path.exists():
                with candidate_path.open("r", encoding="utf-8") as handle:
                    candidate_payload = json.load(handle)
                    candidate_name = candidate_path.name
        if candidate_payload is None:
            return PrometheusResult("vector", [])
        if self.verbose:
            log(f"Loaded evidence from {candidate_name}", verbose=True)
        return parse_prometheus_payload(candidate_payload)


def parse_prometheus_payload(payload: object) -> PrometheusResult:
    if not isinstance(payload, Mapping):
        raise JsonStructureError("Prometheus payload must be a JSON object")
    status = payload.get("status", "success")
    if status != "success" and status is not None:
        raise JsonStructureError(f"Prometheus payload status not success: {status}")
    data = payload.get("data")
    if not isinstance(data, Mapping):
        raise JsonStructureError("Prometheus payload data must be an object")
    result_type = data.get("resultType")
    result = data.get("result", [])
    if not isinstance(result_type, str):
        result_type = "vector"
    if not isinstance(result, Sequence):
        raise JsonStructureError("Prometheus payload result must be an array")
    return PrometheusResult(result_type, result)


def extract_instant_value(sample: Mapping[str, object]) -> Optional[float]:
    value = sample.get("value")
    if isinstance(value, Sequence) and len(value) == 2:
        try:
            return float(value[1])
        except (TypeError, ValueError):
            return None
    return None


def ensure_finite(values: Iterable[float]) -> None:
    for value in values:
        if not is_finite(value):
            raise NumericHealthError("Detected NaN or Inf in evaluated metrics")


def group_by_labels(
    result: PrometheusResult,
    labels: Sequence[str],
) -> Dict[Tuple[str, ...], List[Tuple[Mapping[str, object], float]]]:
    groups: Dict[Tuple[str, ...], List[Tuple[Mapping[str, object], float]]] = {}
    for sample in result.result:
        if not isinstance(sample, Mapping):
            continue
        metric = sample.get("metric")
        if not isinstance(metric, Mapping):
            continue
        numeric = extract_instant_value(sample)
        if numeric is None:
            continue
        key = tuple(str(metric.get(label, "")) for label in labels)
        groups.setdefault(key, []).append((metric, numeric))
    return groups


def run_quality_checks(args: argparse.Namespace) -> int:
    evidence_dir = Path(args.evidence_dir)
    manifest_path = Path(args.manifest) if args.manifest else evidence_dir / "prom_scrape.json"
    evidence_store = EvidenceStore(evidence_dir, verbose=args.verbose) if not args.live else None

    query_source = QuerySource(
        live=args.live,
        addr=args.addr,
        timeout=args.timeout,
        evidence_store=evidence_store,
        verbose=args.verbose,
    )

    window = args.window

    # Fetch datasets
    histogram_increase = query_source.query(
        f"increase(amm_op_latency_seconds_bucket[{window}])",
        alias="hist_increase",
    )
    bucket_rate = query_source.query(
        f"sum by (op,service) (rate(amm_op_latency_seconds_bucket[{window}]))",
        alias="bucket_rate",
    )
    count_rate = query_source.query(
        f"rate(amm_op_latency_seconds_count[{window}])",
        alias="count_rate",
    )
    avg_latency_query = (
        f"rate(amm_op_latency_seconds_sum[{window}]) / "
        f"ignoring(le) rate(amm_op_latency_seconds_count[{window}])"
    )
    avg_latency = query_source.query(avg_latency_query, alias="avg_latency")
    p75_query = (
        "histogram_quantile(0.75, sum by (le,op,service) "
        f"(rate(amm_op_latency_seconds_bucket[{window}])))"
    )
    p75 = query_source.query(p75_query, alias="p75")
    p95_query = (
        "histogram_quantile(0.95, sum by (le,op,service) "
        f"(rate(amm_op_latency_seconds_bucket[{window}])))"
    )
    p95 = query_source.query(p95_query, alias="p95")
    hooks = query_source.query(
        f"sum by (hook_id,status) (rate(hook_executions_total[{window}]))",
        alias="hook_rate",
    )

    telemetry_duration_query = (
        "histogram_quantile(0.95, sum by (le,job) "
        f"(rate(scrape_duration_seconds_bucket[{window}])))"
    )
    telemetry_duration = query_source.query(
        telemetry_duration_query,
        alias="telemetry_duration",
    )
    telemetry_samples_query = (
        f"avg by (job) (rate(scrape_samples_post_metric_relabeling[{window}]))"
    )
    telemetry_samples = query_source.query(
        telemetry_samples_query,
        alias="telemetry_samples",
    )
    telemetry_interval_query = (
        f"avg_over_time(target_interval_length_seconds[{window}])"
    )
    telemetry_interval = query_source.query(
        telemetry_interval_query,
        alias="telemetry_interval",
    )

    # Fail-fast dataset validation
    bucket_groups = group_by_labels(histogram_increase, ("op", "service"))
    bucket_nonzero = any(value > 0 for group in bucket_groups.values() for _, value in group)
    quantile_values = []
    for sample in list(p75.result) + list(p95.result):
        if isinstance(sample, Mapping):
            value = extract_instant_value(sample)
            if value is not None and is_finite(value):
                quantile_values.append(value)
    quantile_available = bool(quantile_values)
    if not bucket_nonzero or not quantile_available:
        raise DatasetMissingError("no usable datasets")

    numeric_values: List[float] = []

    # Histogram monotonicity
    histogram_monotonic = True
    for key, samples in bucket_groups.items():
        ordered = []
        for metric, value in samples:
            le_raw = str(metric.get("le", ""))
            le_value = parse_le(le_raw)
            ordered.append((le_value, value))
            numeric_values.append(value)
        ordered.sort(key=lambda item: item[0])
        previous = -math.inf
        for le_value, value in ordered:
            if value < previous - 1e-9:
                histogram_monotonic = False
                break
            previous = value
        if not histogram_monotonic:
            break

    # Closure check
    bucket_rate_groups = group_by_labels(bucket_rate, ("op", "service"))
    count_rate_groups = group_by_labels(count_rate, ("op", "service"))
    if not bucket_rate_groups or not count_rate_groups:
        raise DatasetMissingError("missing histogram rate data")

    closure_error_pct = 0.0
    closure_ok = True
    for key, bucket_samples in bucket_rate_groups.items():
        bucket_value = bucket_samples[0][1] if bucket_samples else 0.0
        numeric_values.append(bucket_value)
        count_samples = count_rate_groups.get(key, [])
        count_value = count_samples[0][1] if count_samples else 0.0
        if count_samples:
            numeric_values.append(count_value)
        error = abs(bucket_value - count_value) / max(count_value, 1e-9) * 100.0
        closure_error_pct = max(closure_error_pct, error)
        if error > CLOSURE_THRESHOLD:
            closure_ok = False
    # include count values absent from bucket groups for finiteness tracking
    for samples in count_rate_groups.values():
        for _, value in samples:
            numeric_values.append(value)

    # Quantile ordering
    avg_groups = group_by_labels(avg_latency, ("op", "service"))
    p75_groups = group_by_labels(p75, ("op", "service"))
    p95_groups = group_by_labels(p95, ("op", "service"))

    if not avg_groups:
        raise DatasetMissingError("missing latency average data")

    avg_le_p75_le_p95 = True
    p95_le_4x_avg = True
    evaluated_pairs = 0

    combined_keys = set(avg_groups.keys()) | set(p75_groups.keys()) | set(p95_groups.keys())

    for key in combined_keys:
        avg_samples = avg_groups.get(key, [])
        p75_samples = p75_groups.get(key, [])
        p95_samples = p95_groups.get(key, [])
        avg_value = avg_samples[0][1] if avg_samples else None
        p75_value = p75_samples[0][1] if p75_samples else None
        p95_value = p95_samples[0][1] if p95_samples else None
        if avg_value is not None:
            numeric_values.append(avg_value)
        if p75_value is not None:
            numeric_values.append(p75_value)
        if p95_value is not None:
            numeric_values.append(p95_value)
        if p75_value is not None or p95_value is not None:
            evaluated_pairs += 1
        if avg_value is None:
            if (p75_value is not None or p95_value is not None) and args.strict:
                avg_le_p75_le_p95 = False
                p95_le_4x_avg = False
            continue
        if p75_value is not None and p95_value is not None:
            if not (avg_value <= p75_value <= p95_value):
                avg_le_p75_le_p95 = False
        elif args.strict:
            avg_le_p75_le_p95 = False
        if p95_value is not None:
            if avg_value < LOW_TRAFFIC_EPS:
                continue
            if p95_value > 4.0 * avg_value:
                p95_le_4x_avg = False
        elif args.strict:
            p95_le_4x_avg = False
    if evaluated_pairs == 0 and args.strict:
        avg_le_p75_le_p95 = False

    # Counters monotonicity (rate >= 0)
    counters_monotonic = True
    hook_groups = group_by_labels(hooks, ("hook_id", "status"))
    if not hook_groups:
        raise DatasetMissingError("missing hook execution counters")
    for samples in hook_groups.values():
        for _, value in samples:
            numeric_values.append(value)
            if value < -1e-9:
                counters_monotonic = False
    if args.strict and not hook_groups:
        counters_monotonic = False

    # Numeric health
    # Telemetry aggregation
    telemetry_available = False
    telemetry_jobs: Dict[str, Dict[str, float]] = {}

    duration_groups = group_by_labels(telemetry_duration, ("job",))
    samples_groups = group_by_labels(telemetry_samples, ("job",))
    interval_groups = group_by_labels(telemetry_interval, ("job",))

    seen_jobs = set(duration_groups.keys()) | set(samples_groups.keys()) | set(interval_groups.keys())
    if seen_jobs:
        telemetry_available = True
    for job_key in seen_jobs:
        job = job_key[0]
        job_entry: Dict[str, float] = {}
        if job_key in duration_groups and duration_groups[job_key]:
            value = duration_groups[job_key][0][1]
            numeric_values.append(value)
            job_entry["scrape_p95_s"] = value
        if job_key in samples_groups and samples_groups[job_key]:
            value = samples_groups[job_key][0][1]
            numeric_values.append(value)
            job_entry["samples_post_relabel_avg"] = value
        if job_key in interval_groups and interval_groups[job_key]:
            value = interval_groups[job_key][0][1]
            numeric_values.append(value)
            job_entry["interval_length_avg_s"] = value
        if job_entry:
            telemetry_jobs[job] = job_entry
    if not telemetry_available:
        if args.strict:
            raise TelemetryUnavailableError("prometheus telemetry unavailable")

    ensure_finite(numeric_values)

    # Cardinality snapshot
    cardinality_snapshot = {
        "amm_op_latency_seconds_bucket": sum(len(samples) for samples in bucket_groups.values()),
        "amm_op_latency_seconds_sum": len(avg_groups),
        "amm_op_latency_seconds_count": len(count_rate_groups),
        "hook_executions_total": len(hook_groups),
    }

    # Build quality check summary
    quality_checks = {
        "histogram_monotonic": histogram_monotonic,
        "bucket_count_closure_error_pct": {
            "value": round(closure_error_pct, 6),
            "threshold": CLOSURE_THRESHOLD,
        },
        "avg_le_p75_le_p95": avg_le_p75_le_p95,
        "p95_le_4x_avg": p95_le_4x_avg,
        "counters_monotonic": counters_monotonic,
        "no_nan_inf": True,
        "prom_telemetry": {
            "available": telemetry_available,
            "jobs": telemetry_jobs,
        },
    }

    failed_checks: List[str] = []
    if not histogram_monotonic:
        failed_checks.append("histogram_monotonic")
    if not closure_ok:
        failed_checks.append("bucket_count_closure_error_pct")
    if not avg_le_p75_le_p95:
        failed_checks.append("avg_le_p75_le_p95")
    if not p95_le_4x_avg:
        failed_checks.append("p95_le_4x_avg")
    if not counters_monotonic:
        failed_checks.append("counters_monotonic")
    if not telemetry_available and args.strict:
        failed_checks.append("prom_telemetry")

    manifest_payload = {
        "quality_checks": quality_checks,
        "cardinality_snapshot": cardinality_snapshot,
    }

    write_manifest(manifest_path, manifest_payload, dry_run=args.dry_run, verbose=args.verbose)

    if failed_checks:
        raise CheckFailedError(
            "; ".join(f"check failed: {name}" for name in failed_checks)
        )

    return 0


class QuerySource:
    """Unified interface for Prometheus queries (live or offline)."""

    def __init__(
        self,
        *,
        live: bool,
        addr: str,
        timeout: float,
        evidence_store: Optional[EvidenceStore],
        verbose: bool,
    ) -> None:
        self.live = live
        self.addr = addr
        self.timeout = timeout
        self.evidence_store = evidence_store
        self.verbose = verbose

    def query(self, query: str, alias: Optional[str] = None) -> PrometheusResult:
        if self.live:
            return self._query_live(query)
        if self.evidence_store is None:
            raise DatasetMissingError("Evidence store not initialised")
        result = self.evidence_store.load_query(query, alias=alias)
        if not result:
            if self.verbose:
                log(
                    f"No offline evidence found for query '{query}' (alias={alias})",
                    verbose=True,
                )
        return result

    def _query_live(self, query: str) -> PrometheusResult:
        base = self.addr
        if base.startswith(":"):
            base = f"http://127.0.0.1{base}"
        elif not base.startswith("http://") and not base.startswith("https://"):
            base = f"http://{base}"
        url = base.rstrip("/") + "/api/v1/query"
        headers = {"Accept": "application/json"}
        try:
            response = requests.get(
                url,
                params={"query": query},
                headers=headers,
                timeout=(CONNECT_TIMEOUT, self.timeout),
            )
            response.raise_for_status()
        except requests.RequestException as exc:
            raise HttpQueryError(f"HTTP error querying Prometheus: {exc}") from exc
        try:
            payload = response.json()
        except ValueError as exc:  # pragma: no cover - network guard
            raise JsonStructureError("Invalid JSON returned by Prometheus") from exc
        if self.verbose:
            log(f"Fetched live query: {query}", verbose=True)
        return parse_prometheus_payload(payload)


def read_manifest(path: Path) -> MutableMapping[str, object]:
    if not path.exists():
        return {}
    try:
        with path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    except json.JSONDecodeError as exc:
        raise JsonStructureError(f"Invalid JSON in manifest: {path}") from exc


def merge_manifest(
    existing: MutableMapping[str, object],
    updates: Mapping[str, object],
) -> MutableMapping[str, object]:
    merged = dict(existing)
    merged.update(updates)
    return merged


def write_manifest(
    path: Path,
    payload: Mapping[str, object],
    *,
    dry_run: bool,
    verbose: bool,
) -> None:
    existing = read_manifest(path)
    merged = merge_manifest(existing, payload)
    if dry_run:
        json.dump(merged, sys.stdout, indent=2, sort_keys=True, ensure_ascii=False)
        sys.stdout.write("\n")
        return
    if not path.parent.exists():
        path.parent.mkdir(parents=True, exist_ok=True)
    temp_path = path.with_suffix(path.suffix + ".tmp")
    with temp_path.open("w", encoding="utf-8") as handle:
        json.dump(merged, handle, indent=2, sort_keys=True, ensure_ascii=False)
        handle.write("\n")
    os.replace(temp_path, path)
    os.chmod(path, 0o644)
    if verbose:
        log(f"Wrote manifest to {path}", verbose=True)


def main(argv: Optional[Sequence[str]] = None) -> int:
    args = parse_args(argv)
    try:
        return run_quality_checks(args)
    except DatasetMissingError as exc:
        log(str(exc), error=True)
        return 6
    except HttpQueryError as exc:
        log(str(exc), error=True)
        return 9
    except JsonStructureError as exc:
        log(str(exc), error=True)
        return 10
    except NumericHealthError as exc:
        log(str(exc), error=True)
        return 11
    except TelemetryUnavailableError as exc:
        log(str(exc), error=True)
        return 12
    except CheckFailedError as exc:
        log(str(exc), error=True)
        return 8
    except QualityCheckError as exc:
        log(str(exc), error=True)
        return 8


if __name__ == "__main__":
    sys.exit(main())
