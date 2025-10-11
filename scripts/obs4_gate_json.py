#!/usr/bin/env python3
"""Summarise OBS-4 trace evidence for local gates."""
from __future__ import annotations

import json
import sys
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Union

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = ROOT / "out" / "obs_gatecheck" / "evidence"
SAMPLE_PATH = EVIDENCE_DIR / "traces_sample.json"
RAW_PATH = EVIDENCE_DIR / "traces_raw.json"

JsonType = Union[Dict[str, Any], List[Any]]


def _load_json(path: Path) -> Optional[JsonType]:
    try:
        text = path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return None
    if not text.strip():
        return None
    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        sys.stderr.write(f"[obs4_gate_json] failed to decode {path}: {exc}\n")
        return None


def _iter_spans(payload: JsonType, depth: int = 0) -> Iterable[Dict[str, Any]]:
    if depth > 4:
        return
    if isinstance(payload, list):
        for item in payload:
            if isinstance(item, dict):
                yield item
            elif isinstance(item, (list, dict)):
                yield from _iter_spans(item, depth + 1)
        return
    if isinstance(payload, dict):
        for key in ("spans", "traces", "data", "records", "span_records"):
            value = payload.get(key)
            if isinstance(value, list):
                for item in value:
                    if isinstance(item, dict):
                        yield item
                    elif isinstance(item, (list, dict)):
                        yield from _iter_spans(item, depth + 1)
                return
        for value in payload.values():
            if isinstance(value, (dict, list)):
                yield from _iter_spans(value, depth + 1)


def _coerce_number(value: Any) -> Optional[float]:
    if isinstance(value, (int, float)):
        return float(value)
    if isinstance(value, str):
        try:
            return float(value.strip())
        except ValueError:
            return None
    return None


def _span_has_error(span: Dict[str, Any]) -> bool:
    status = span.get("status") or span.get("status_code") or span.get("statusCode")
    if isinstance(status, str) and "error" in status.lower():
        return True
    if isinstance(status, dict):
        for value in status.values():
            if isinstance(value, str) and "error" in value.lower():
                return True
    attributes = span.get("attributes")
    if isinstance(attributes, dict):
        for key, value in attributes.items():
            key_l = str(key).lower()
            if "error" in key_l:
                if isinstance(value, bool) and value:
                    return True
                if isinstance(value, str) and value.lower() not in ("", "false", "0"):
                    return True
            if key_l == "status" and isinstance(value, str) and "error" in value.lower():
                return True
    events = span.get("events")
    if isinstance(events, list):
        for event in events:
            if not isinstance(event, dict):
                continue
            name = str(event.get("name", "")).lower()
            if "exception" in name or "error" in name:
                return True
            attrs = event.get("attributes")
            if isinstance(attrs, dict):
                for key, value in attrs.items():
                    key_l = str(key).lower()
                    if "error" in key_l:
                        return True
                    if isinstance(value, str) and "error" in value.lower():
                        return True
    return False


def _slow_threshold(sample: Optional[JsonType]) -> float:
    candidates: List[float] = []

    def collect(obj: Any) -> None:
        if isinstance(obj, dict):
            for key, value in obj.items():
                key_l = str(key).lower()
                if any(
                    token in key_l
                    for token in (
                        "slow_sampling_ms",
                        "slow_threshold_ms",
                        "slow_ms",
                        "latency_threshold_ms",
                        "threshold_ms",
                    )
                ):
                    number = _coerce_number(value)
                    if number is not None:
                        candidates.append(number)
                collect(value)
        elif isinstance(obj, list):
            for item in obj:
                collect(item)

    if sample is not None:
        collect(sample)
    return candidates[0] if candidates else 100.0


def _span_is_slow(span: Dict[str, Any], threshold: float) -> bool:
    attributes = span.get("attributes")
    if isinstance(attributes, dict):
        for key, value in attributes.items():
            key_l = str(key).lower()
            if any(token in key_l for token in ("slow", "latency", "duration", "elapsed")):
                number = _coerce_number(value)
                if number is not None and number >= threshold:
                    return True
    events = span.get("events")
    if isinstance(events, list):
        for event in events:
            if not isinstance(event, dict):
                continue
            name = str(event.get("name", "")).lower()
            if "slow" in name or "latency" in name:
                return True
            attrs = event.get("attributes")
            if isinstance(attrs, dict):
                for key, value in attrs.items():
                    key_l = str(key).lower()
                    if any(token in key_l for token in ("duration", "slow", "latency", "elapsed")):
                        number = _coerce_number(value)
                        if number is not None and number >= threshold:
                            return True
    return False


def _has_links(spans: Iterable[Dict[str, Any]]) -> bool:
    by_trace: Dict[str, List[str]] = {}
    spans_list = list(spans)
    for span in spans_list:
        name = str(span.get("name", "")).lower()
        links = span.get("links")
        if isinstance(links, list):
            for link in links:
                target = None
                if isinstance(link, dict):
                    for key in ("target", "span", "span_name", "scope", "to", "related"):
                        if key in link:
                            target = str(link[key])
                            break
                else:
                    target = str(link)
                if target:
                    target_l = target.lower()
                    if ("amm" in name and "cdc" in target_l) or (
                        "cdc" in name and "amm" in target_l
                    ):
                        return True
        trace_id = span.get("trace_id") or span.get("traceId")
        if trace_id:
            trace_key = str(trace_id)
            by_trace.setdefault(trace_key, []).append(name)
    for names in by_trace.values():
        has_amm = any("amm" in name for name in names)
        has_cdc = any("cdc" in name for name in names)
        if has_amm and has_cdc:
            return True
    return False


def _has_correlation(spans: Iterable[Dict[str, Any]], sample: Optional[JsonType]) -> bool:
    for span in spans:
        trace_id = span.get("trace_id") or span.get("traceId")
        span_id = span.get("span_id") or span.get("spanId")
        if trace_id and span_id:
            return True
        attributes = span.get("attributes")
        if isinstance(attributes, dict):
            if attributes.get("trace_id") and attributes.get("span_id"):
                return True
    if isinstance(sample, dict):
        for key in ("correlation", "meta", "summary"):
            block = sample.get(key)
            if isinstance(block, dict):
                if block.get("available") or block.get("ok"):
                    return True
                fields = block.get("fields") or block.get("log_fields") or block.get("required_fields")
                if isinstance(fields, list):
                    lowered = {str(item).lower() for item in fields}
                    if "trace_id" in lowered and "span_id" in lowered:
                        return True
    return False


def _trace_count_hint(raw_payload: Optional[JsonType]) -> Optional[int]:
    if raw_payload is None:
        return None
    if isinstance(raw_payload, list):
        return len(raw_payload)
    if isinstance(raw_payload, dict):
        for key in ("traces", "data", "records", "spans"):
            value = raw_payload.get(key)
            if isinstance(value, list):
                return len(value)
        for key in ("trace_count", "count"):
            value = raw_payload.get(key)
            number = _coerce_number(value)
            if number is not None:
                return int(number)
    return None


def main() -> int:
    sample = _load_json(SAMPLE_PATH)
    spans = list(_iter_spans(sample)) if sample is not None else []

    has_error = any(_span_has_error(span) for span in spans)
    threshold = _slow_threshold(sample)
    has_slow = any(_span_is_slow(span, threshold) for span in spans)
    has_links = _has_links(spans)
    has_corr = _has_correlation(spans, sample)
    spans_present = bool(spans)

    raw_payload = _load_json(RAW_PATH)
    trace_hint = _trace_count_hint(raw_payload)

    ok = spans_present and has_error and has_slow and has_links and has_corr

    result = {
        "ok": ok,
        "has_error": has_error,
        "has_slow": has_slow,
        "has_links_cdc_amm": has_links,
        "trace_count_hint": trace_hint,
    }
    json.dump(result, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")

    return 0 if ok else 3


if __name__ == "__main__":
    sys.exit(main())
