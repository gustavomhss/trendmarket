#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple
import xml.etree.ElementTree as ET

SEVERITY_PATTERN = re.compile(r"\\b\\[?(P1|P2|P3)\\]?\\b", re.IGNORECASE)
SEED_PATTERNS: Sequence[re.Pattern[str]] = (
    re.compile(r"seed\\s*=\\s*(\\d{4,})", re.IGNORECASE),
    re.compile(r"seed[:=]\\s*(\\d{4,})", re.IGNORECASE),
    re.compile(r"re-?run with (?:--)?seed[=:]?(\\d{4,})", re.IGNORECASE),
    re.compile(r"minimal failing.*\\bseed\\b[:=]\\s*(\\d{4,})", re.IGNORECASE),
)
SEVERITY_ORDER = {"P1": 0, "P2": 1, "P3": 2}
MAX_JSON_MESSAGE = 512
MAX_MD_MESSAGE = 120


@dataclass
class FailureItem:
    suite: str
    name: str
    severity: str
    time: float
    message: str
    seed: Optional[str] = None


def truncate(value: str, limit: int) -> str:
    if len(value) <= limit:
        return value
    if limit <= 1:
        return value[:limit]
    return value[: limit - 1] + "…"


def sanitize_line(value: str) -> str:
    value = value.replace("\r", " ")
    value = value.replace("\n", " ")
    value = re.sub(r"\s+", " ", value)
    return value.strip()


def determine_severity(name: str, classname: str) -> str:
    for target in (name, classname):
        match = SEVERITY_PATTERN.search(target or "")
        if match:
            return match.group(1).upper()
    return "P2"


def parse_time(value: Optional[str]) -> float:
    if not value:
        return 0.0
    try:
        return float(value)
    except ValueError:
        return 0.0


def extract_failure_message(node: ET.Element) -> str:
    if node.text and node.text.strip():
        return node.text
    message_attr = node.get("message")
    if message_attr:
        return message_attr
    return ""


def build_failure_item(
    suite: str,
    testcase: ET.Element,
    failure_node: ET.Element,
) -> FailureItem:
    name = testcase.get("name", "")
    classname = testcase.get("classname", "")
    severity = determine_severity(name, classname)
    time_value = parse_time(testcase.get("time"))
    raw_message = extract_failure_message(failure_node)
    if raw_message:
        first_line = sanitize_line(raw_message.splitlines()[0])
    else:
        first_line = sanitize_line(failure_node.get("message", ""))
    if not first_line:
        first_line = "No failure message provided."
    return FailureItem(
        suite=suite or "",
        name=name or "",
        severity=severity,
        time=time_value,
        message=first_line,
    )


def parse_junit_report(path: Path) -> Tuple[List[FailureItem], Optional[str]]:
    try:
        tree = ET.parse(path)
    except Exception as exc:  # noqa: BLE001
        message = f"Failed to parse JUnit report: {exc}"
        synthetic = FailureItem(
            suite="JUnitParser",
            name="report", 
            severity="P2",
            time=0.0,
            message=sanitize_line(message),
        )
        return [synthetic], message

    root = tree.getroot()
    failures: List[FailureItem] = []

    for testsuite in root.iter("testsuite"):
        suite_name = testsuite.get("name", "")
        for testcase in testsuite.findall("testcase"):
            failure_node = testcase.find("failure")
            error_node = testcase.find("error") if failure_node is None else None
            node = failure_node or error_node
            if node is None:
                continue
            failures.append(build_failure_item(suite_name, testcase, node))

    return failures, None


def read_text_safe(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except Exception:  # noqa: BLE001
        return ""


def write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def collect_seed_matches(lines: Sequence[str]) -> List[Tuple[int, str]]:
    results: List[Tuple[int, str]] = []
    for index, line in enumerate(lines):
        for pattern in SEED_PATTERNS:
            for match in pattern.finditer(line):
                seed = match.group(1)
                results.append((index, seed))
    return results


def map_seeds_to_failures(
    log_path: Optional[Path],
    failures: Sequence[FailureItem],
) -> Dict[str, str]:
    if not log_path:
        return {}
    if not log_path.exists():
        return {}

    content = read_text_safe(log_path)
    if not content:
        return {}

    lines = content.splitlines()
    matches = collect_seed_matches(lines)
    if not matches:
        return {}

    mapping: Dict[str, Tuple[int, str]] = {}
    for failure in failures:
        best: Optional[Tuple[int, str]] = None
        for index, seed in matches:
            contexts = [lines[index]]
            if index > 0:
                contexts.append(lines[index - 1])
            if index + 1 < len(lines):
                contexts.append(lines[index + 1])
            match_found = any(
                failure.name and failure.name in context or failure.suite and failure.suite in context
                for context in contexts
            )
            if not match_found:
                continue
            if best is None or index > best[0]:
                best = (index, seed)
        if best is not None:
            mapping[f"{failure.suite}::{failure.name}"] = best[1]
    return {key: value for key, value in mapping.items()}


def apply_seeds(failures: Iterable[FailureItem], seeds: Dict[str, str]) -> None:
    for failure in failures:
        key = f"{failure.suite}::{failure.name}"
        seed = seeds.get(key)
        if seed:
            failure.seed = seed


def sort_failures(failures: List[FailureItem]) -> List[FailureItem]:
    def sort_key(item: FailureItem) -> Tuple[int, str, str]:
        severity_rank = SEVERITY_ORDER.get(item.severity, SEVERITY_ORDER["P2"])
        return (severity_rank, item.suite, item.name)

    return sorted(failures, key=sort_key)


def compute_counts(failures: Sequence[FailureItem]) -> Dict[str, int]:
    counts = {"P1": 0, "P2": 0, "P3": 0}
    for failure in failures:
        if failure.severity in counts:
            counts[failure.severity] += 1
        else:
            counts["P2"] += 1
    return counts


def determine_gate_status(counts: Dict[str, int]) -> str:
    if counts.get("P1", 0) > 0 or counts.get("P2", 0) > 0:
        return "fail"
    return "pass"


def build_json_summary(
    gate_status: str,
    counts: Dict[str, int],
    failures: Sequence[FailureItem],
    generated_at: str,
) -> str:
    meta = {
        "sha": os.getenv("GITHUB_SHA", ""),
        "ref": os.getenv("GITHUB_REF", ""),
        "run_id": os.getenv("GITHUB_RUN_ID", ""),
        "generated_at": generated_at,
    }
    failed_entries: List[Dict[str, object]] = []
    for item in failures:
        entry: Dict[str, object] = {
            "suite": item.suite,
            "name": item.name,
            "severity": item.severity,
            "time": item.time,
            "message": truncate(item.message, MAX_JSON_MESSAGE),
        }
        if item.seed:
            entry["seed"] = item.seed
        failed_entries.append(entry)

    payload = {
        "gate_status": gate_status,
        "counts": counts,
        "failed": failed_entries,
        "meta": meta,
    }
    return json.dumps(payload, indent=2, sort_keys=False) + "\n"


def escape_md(value: str) -> str:
    escaped = value.replace("|", "\\|")
    escaped = escaped.replace("`", "\\`")
    escaped = escaped.replace("<", "&lt;").replace(">", "&gt;")
    return escaped


def format_time(value: float) -> str:
    return f"{value:.3f}"


def build_failures_table(failures: Sequence[FailureItem]) -> List[str]:
    lines = ["| Severity | Suite | Test | Time(s) | Seed | Message |", "| --- | --- | --- | --- | --- | --- |"]
    for item in failures:
        seed_display = item.seed or ""
        message_display = truncate(item.message, MAX_MD_MESSAGE)
        row = "| {severity} | {suite} | {name} | {time} | {seed} | {message} |".format(
            severity=escape_md(item.severity),
            suite=escape_md(item.suite),
            name=escape_md(item.name),
            time=escape_md(format_time(item.time)),
            seed=escape_md(seed_display),
            message=escape_md(message_display),
        )
        lines.append(row)
    return lines


def build_repro_section(failures: Sequence[FailureItem]) -> List[str]:
    lines: List[str] = []
    for item in failures:
        if not item.seed:
            continue
        command = f"PROPTEST_SEED={item.seed} cargo test -- {item.name}"
        lines.append(command)
    return lines


def build_markdown_summary(
    gate_status: str,
    counts: Dict[str, int],
    failures: Sequence[FailureItem],
    generated_at: str,
) -> str:
    status_label = gate_status.upper()
    lines = ["# A110 — Invariants Gate Summary", f"Status: {status_label}"]
    lines.append("")
    lines.append("## Counts")
    lines.append(f"- P1: {counts['P1']}")
    lines.append(f"- P2: {counts['P2']}")
    lines.append(f"- P3: {counts['P3']}")
    lines.append("")
    rule_desc = (
        "Gate status is FAIL because P1/P2 failures are blocking." if gate_status == "fail" else "Gate status is PASS because only P3 or no failures were detected."
    )
    lines.append(rule_desc)
    lines.append("")

    if failures:
        lines.append("## Failing Tests")
        lines.extend(build_failures_table(failures))
        lines.append("")
    else:
        lines.append("No failing tests detected.")
        lines.append("")

    repro = build_repro_section(failures)
    if repro:
        lines.append("## Como reproduzir")
        for command in repro:
            lines.append(f"- `{command}`")
        lines.append("")

    meta_line = " | ".join(
        [
            f"SHA: {os.getenv('GITHUB_SHA', '')}",
            f"Ref: {os.getenv('GITHUB_REF', '')}",
            f"Run ID: {os.getenv('GITHUB_RUN_ID', '')}",
            f"Generated at: {generated_at}",
        ]
    )
    lines.append("---")
    lines.append(meta_line)
    lines.append("")
    return "\n".join(lines)


def parse_args(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Generate A110 invariants gate summary.")
    parser.add_argument("--junit", required=True, help="Path to the JUnit XML report.")
    parser.add_argument("--log", help="Path to the cargo test log for seed extraction.")
    parser.add_argument("--summary-json", required=True, help="Path to write the JSON summary.")
    parser.add_argument("--summary-md", required=True, help="Path to write the Markdown summary.")
    return parser.parse_args(argv)


def main(argv: Optional[Sequence[str]] = None) -> None:
    args = parse_args(argv)
    junit_path = Path(args.junit)
    log_path = Path(args.log) if args.log else None
    json_path = Path(args.summary_json)
    md_path = Path(args.summary_md)

    failures, parse_error = parse_junit_report(junit_path)
    if parse_error:
        print(f"[a110_gate] {parse_error}", file=sys.stderr)

    seeds = map_seeds_to_failures(log_path, failures)
    apply_seeds(failures, seeds)

    ordered_failures = sort_failures(list(failures))
    counts = compute_counts(ordered_failures)
    gate_status = determine_gate_status(counts)
    generated_at = datetime.now(timezone.utc).isoformat()

    json_summary = build_json_summary(gate_status, counts, ordered_failures, generated_at)
    markdown_summary = build_markdown_summary(gate_status, counts, ordered_failures, generated_at)

    write_text(json_path, json_summary)
    write_text(md_path, markdown_summary)

    sys.exit(0)


if __name__ == "__main__":
    main()
