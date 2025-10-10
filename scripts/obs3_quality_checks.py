#!/usr/bin/env python3
"""OBS-3 quality validator for Prometheus evidence bundles."""
from __future__ import annotations

import argparse
import json
import sys
from collections import defaultdict
from collections.abc import Iterable
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

MetricLabels = tuple[tuple[str, str], ...]
TimeSeries = list[list[str]]
HistogramMap = dict[MetricLabels, list[tuple[float, TimeSeries]]]


def read_json(path: Path) -> dict[str, Any]:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def load_instant(dir_path: Path) -> dict[str, list[dict[str, Any]]]:
    result: dict[str, list[dict[str, Any]]] = {}
    if not dir_path.exists():
        return result
    for file in sorted(dir_path.glob("*.json")):
        payload = read_json(file)
        if payload.get("status") != "success":
            continue
        result[file.name] = payload["data"].get("result", [])
    return result


def load_range(dir_path: Path) -> dict[str, list[dict[str, Any]]]:
    result: dict[str, list[dict[str, Any]]] = {}
    if not dir_path.exists():
        return result
    for file in sorted(dir_path.glob("*.json")):
        payload = read_json(file)
        if payload.get("status") != "success":
            continue
        result[file.name] = payload["data"].get("result", [])
    return result


def ensure_monotonic(series: TimeSeries) -> bool:
    last = float("-inf")
    for _, value in series:
        sample = float(value)
        if sample + 1e-9 < last:
            return False
        last = sample
    return True


def build_histogram_map(bucket_results: list[dict[str, Any]]) -> HistogramMap:
    histograms: HistogramMap = {}
    for entry in bucket_results:
        metric = entry.get("metric", {})
        values = entry.get("values", [])
        le_raw = metric.get("le")
        if le_raw is None:
            continue
        if le_raw == "+Inf":
            le_value = float("inf")
        else:
            le_value = float(le_raw)
        key = tuple(sorted((k, v) for k, v in metric.items() if k not in {"__name__", "le"}))
        histograms.setdefault(key, []).append((le_value, values))
    # sort buckets by upper bound
    for _labels, buckets in histograms.items():
        buckets.sort(key=lambda item: item[0])
    return histograms


def closure_ok(
    histograms: HistogramMap,
    count_results: list[dict[str, Any]],
) -> tuple[bool, list[str]]:
    count_latest: dict[MetricLabels, float] = {}
    for entry in count_results:
        metric = tuple(
            sorted((k, v) for k, v in entry.get("metric", {}).items() if k != "__name__")
        )
        values = entry.get("values", [])
        if not values:
            continue
        count_latest[metric] = float(values[-1][1])
    violations: list[str] = []
    for key, buckets in histograms.items():
        if not buckets:
            continue
        last_bucket = buckets[-1][1]
        if not last_bucket:
            continue
        final_bucket_value = float(last_bucket[-1][1])
        count_value = count_latest.get(key)
        if count_value is None:
            continue
        if count_value == 0:
            continue
        diff = abs(final_bucket_value - count_value) / count_value
        if diff > 0.03:
            label_str = ",".join(f"{k}={v}" for k, v in dict(key).items())
            violations.append(f"closure diff {diff:.4f} for {label_str}")
    return (len(violations) == 0, violations)


def check_histograms(histograms: HistogramMap) -> tuple[bool, list[str]]:
    failures: list[str] = []
    for key, buckets in histograms.items():
        bucket_len = None
        for idx, (_, series) in enumerate(buckets):
            if not ensure_monotonic(series):
                label_str = ",".join(f"{k}={v}" for k, v in dict(key).items())
                failures.append(f"non-monotonic bucket le={buckets[idx][0]} labels={label_str}")
            if bucket_len is None:
                bucket_len = len(series)
            else:
                bucket_len = min(bucket_len, len(series))
        if bucket_len is None:
            continue
        # cross-bucket monotonicity per timestamp
        for i in range(bucket_len):
            last_value = float("-inf")
            for upper, series in buckets:
                sample = float(series[i][1])
                if sample + 1e-9 < last_value:
                    label_str = ",".join(f"{k}={v}" for k, v in dict(key).items())
                    failures.append(
                        "bucket ordering broke at t="
                        f"{series[i][0]} le={upper} labels={label_str}"
                    )
                    break
                last_value = sample
    return (len(failures) == 0, failures)


def check_counters(counter_results: Iterable[dict[str, Any]]) -> tuple[bool, list[str]]:
    failures: list[str] = []
    for entry in counter_results:
        metric = entry.get("metric", {})
        name = metric.get("__name__", "metric")
        series = entry.get("values", [])
        if not ensure_monotonic(series):
            label_str = ",".join(f"{k}={v}" for k, v in metric.items() if k != "__name__")
            failures.append(f"counter {name} not monotonic ({label_str})")
    return (len(failures) == 0, failures)


def collect_quantiles(
    instant: dict[str, list[dict[str, Any]]]
) -> dict[tuple[str, str], dict[str, float]]:
    quantiles: dict[tuple[str, str], dict[str, float]] = defaultdict(dict)
    mapping = {
        "latency_p75.json": "p75",
        "latency_p95.json": "p95",
        "latency_avg.json": "avg",
    }
    for filename, key in mapping.items():
        for sample in instant.get(filename, []):
            metric = sample.get("metric", {})
            value = float(sample.get("value", [0, "0"])[1])
            ident = (metric.get("service", "unknown"), metric.get("op", ""))
            quantiles[ident][key] = value
    return quantiles


def validate_quantiles(
    quantiles: dict[tuple[str, str], dict[str, float]]
) -> tuple[bool, list[str]]:
    failures: list[str] = []
    for ident, values in quantiles.items():
        service, op = ident
        avg = values.get("avg")
        p75 = values.get("p75")
        p95 = values.get("p95")
        if avg is None or p75 is None or p95 is None:
            failures.append(f"missing quantile for service={service} op={op}")
            continue
        if not (avg - 1e-9 <= p75 + 1e-9 and p75 - 1e-9 <= p95 + 1e-9):
            failures.append(
                "ordering violation service="
                f"{service} op={op}: avg={avg} p75={p75} p95={p95}"
            )
        if p95 > 4 * avg + 1e-9:
            failures.append(
                f"p95 exceeds 4x avg for service={service} op={op}: avg={avg} p95={p95}"
            )
    return (len(failures) == 0, failures)


def detect_nan_inf(
    range_payloads: Iterable[list[dict[str, Any]]],
    instant_payloads: Iterable[list[dict[str, Any]]],
) -> tuple[bool, list[str]]:
    failures: list[str] = []

    def inspect_value(metric: dict[str, Any], value: str) -> None:
        if value.lower() in {"nan", "inf", "+inf", "-inf"}:
            label_str = ",".join(f"{k}={v}" for k, v in metric.items())
            failures.append(f"invalid numeric value {value} for {label_str}")
    for dataset in range_payloads:
        for entry in dataset:
            metric = entry.get("metric", {})
            for _, value in entry.get("values", []):
                inspect_value(metric, value)
    for dataset in instant_payloads:
        for entry in dataset:
            metric = entry.get("metric", {})
            value = entry.get("value", [None, "0"])[1]
            inspect_value(metric, value)
    return (len(failures) == 0, failures)


def snapshot_cardinality(histograms: HistogramMap) -> dict[str, Any]:
    totals: dict[str, int] = defaultdict(int)
    total_series = 0
    for key in histograms:
        labels = dict(key)
        service = labels.get("service", "unknown")
        totals[service] += 1
        total_series += 1
    return {
        "total_series": total_series,
        "by_service": dict(sorted(totals.items())),
    }


def write_report(output: Path, log_path: Path | None, payload: dict[str, Any]) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    with output.open("w", encoding="utf-8") as handle:
        json.dump(payload, handle, indent=2, sort_keys=True)
        handle.write("\n")
    if log_path is not None:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with log_path.open("w", encoding="utf-8") as handle:
            handle.write("OBS-3 Quality Checks Summary\n")
            for name, section in payload["checks"].items():
                handle.write(f"- {name}: {'PASS' if section['ok'] else 'FAIL'}\n")


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description="Run OBS-3 Prometheus quality checks")
    parser.add_argument("--instant-dir", required=True, type=Path)
    parser.add_argument("--range-dir", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--log", type=Path)
    args = parser.parse_args(argv)

    instant_payloads = load_instant(args.instant_dir)
    range_payloads = load_range(args.range_dir)

    bucket_results = range_payloads.get("amm_op_latency_seconds_bucket.json", [])
    count_results = range_payloads.get("amm_op_latency_seconds_count.json", [])
    sum_results = range_payloads.get("amm_op_latency_seconds_sum.json", [])
    hook_results = range_payloads.get("amm_hook_invocations_total.json", [])

    histograms = build_histogram_map(bucket_results)
    histogram_ok, histogram_failures = check_histograms(histograms)
    closure_ok_flag, closure_failures = closure_ok(histograms, count_results)
    counter_ok_flag, counter_failures = check_counters(count_results + sum_results + hook_results)
    quantiles = collect_quantiles(instant_payloads)
    quantile_ok_flag, quantile_failures = validate_quantiles(quantiles)
    nan_ok, nan_failures = detect_nan_inf(range_payloads.values(), instant_payloads.values())
    cardinality = snapshot_cardinality(histograms)

    report = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "checks": {
            "histogram_monotonic": {"ok": histogram_ok, "details": histogram_failures},
            "closure_within_3pct": {"ok": closure_ok_flag, "details": closure_failures},
            "quantile_ordering": {"ok": quantile_ok_flag, "details": quantile_failures},
            "counter_monotonic": {"ok": counter_ok_flag, "details": counter_failures},
            "nan_inf_zero": {"ok": nan_ok, "details": nan_failures},
        },
        "cardinality_snapshot": cardinality,
        "quantiles": quantiles,
    }

    all_ok = all(section["ok"] for section in report["checks"].values())

    write_report(args.output, args.log, report)

    return 0 if all_ok else 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
