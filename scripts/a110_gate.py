#!/usr/bin/env python3
"""A110 gate script for summarizing test failures."""

from __future__ import annotations

import argparse
import json
import logging
import os
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Sequence, Tuple
import xml.etree.ElementTree as ET

SEVERITY_PATTERN = re.compile(r"\b\[?(P[123])\]?\b", re.IGNORECASE)
SEVERITY_ORDER = {"P1": 0, "P2": 1, "P3": 2}
DEFAULT_SEVERITY = "P2"
MESSAGE_TRUNCATE_AT = 200


@dataclass(order=True)
class FailureRecord:
  sort_index: Tuple[int, str, str] = field(init=False, repr=False)
  suite: str
  name: str
  severity: str
  time: float
  message: str
  seed: Optional[str] = None

  def __post_init__(self) -> None:
    severity_rank = SEVERITY_ORDER.get(self.severity, len(SEVERITY_ORDER))
    self.sort_index = (severity_rank, self.suite, self.name)


def parse_arguments(argv: Optional[Sequence[str]] = None) -> argparse.Namespace:
  parser = argparse.ArgumentParser(description="Summarize test results for A110 gate.")
  parser.add_argument("--junit", required=True, type=Path, help="Path to junit.xml file")
  parser.add_argument("--log", type=Path, help="Path to cargo test log (optional)")
  parser.add_argument(
    "--summary-json",
    required=True,
    type=Path,
    help="Path to output JSON summary",
  )
  parser.add_argument(
    "--summary-md",
    required=True,
    type=Path,
    help="Path to output Markdown summary",
  )
  return parser.parse_args(argv)


def detect_severity(*candidates: Optional[str]) -> str:
  for candidate in candidates:
    if not candidate:
      continue
    match = SEVERITY_PATTERN.search(candidate)
    if match:
      return match.group(1).upper()
  return DEFAULT_SEVERITY


def safe_float(value: Optional[str]) -> float:
  if value is None:
    return 0.0
  try:
    return float(value)
  except (TypeError, ValueError):
    return 0.0


def extract_message(elements: Iterable[ET.Element]) -> str:
  messages: List[str] = []
  for element in elements:
    if element is None:
      continue
    text_parts: List[str] = []
    message_attr = element.get("message")
    if message_attr:
      text_parts.append(message_attr)
    if element.text:
      text_parts.append(element.text)
    if element.tail:
      text_parts.append(element.tail)
    combined = "\n".join(part.strip() for part in text_parts if part)
    if combined:
      messages.append(combined)
  return "\n".join(messages)


def parse_testcases(root: ET.Element) -> List[FailureRecord]:
  failures: List[FailureRecord] = []
  for testcase in root.iter("testcase"):
    failure_elements = list(testcase.findall("failure"))
    error_elements = list(testcase.findall("error"))
    if not failure_elements and not error_elements:
      continue
    suite = testcase.get("classname") or testcase.get("class") or ""
    name = testcase.get("name") or "<unknown>"
    severity = detect_severity(suite, name)
    time_value = safe_float(testcase.get("time"))
    message = extract_message(failure_elements + error_elements)
    failures.append(
      FailureRecord(
        suite=suite or "<unspecified>",
        name=name,
        severity=severity,
        time=time_value,
        message=message,
      )
    )
  return failures


def parse_junit(path: Path) -> Tuple[List[FailureRecord], bool]:
  try:
    tree = ET.parse(path)
  except ET.ParseError as exc:
    logging.error("Invalid JUnit XML at %s: %s", path, exc)
    synthetic = FailureRecord(
      suite="<junit>",
      name="invalid-junit-xml",
      severity="P2",
      time=0.0,
      message=f"Invalid JUnit XML: {exc}",
    )
    return [synthetic], False
  except OSError as exc:
    logging.error("Failed to read JUnit XML at %s: %s", path, exc)
    synthetic = FailureRecord(
      suite="<junit>",
      name="unreadable-junit-xml",
      severity="P2",
      time=0.0,
      message=f"Failed to read JUnit XML: {exc}",
    )
    return [synthetic], False
  root = tree.getroot()
  failures = parse_testcases(root)
  return failures, True


def normalize_whitespace(text: str) -> str:
  return " ".join(text.split())


def truncate_message(text: str, limit: int = MESSAGE_TRUNCATE_AT) -> str:
  if len(text) <= limit:
    return text
  return text[: limit - 1] + "…"


def ensure_parent(path: Path) -> None:
  path.parent.mkdir(parents=True, exist_ok=True)


def parse_proptest_log(log_path: Path) -> Dict[str, str]:
  seed_map: Dict[str, str] = {}
  if not log_path.exists():
    logging.warning("Log file %s does not exist", log_path)
    return seed_map
  test_name_pattern = re.compile(r"^test\s+([^\s]+)")
  section_pattern = re.compile(r"^----\s+(.+?)\s+stdout\s+----")
  seed_patterns = [
    re.compile(r"seed\s*[=:]\s*([0-9]+)", re.IGNORECASE),
    re.compile(r"--seed=([0-9]+)"),
  ]
  current_test: Optional[str] = None
  try:
    with log_path.open("r", encoding="utf-8", errors="replace") as handle:
      for raw_line in handle:
        line = raw_line.strip()
        if not line:
          continue
        match_test = test_name_pattern.match(line)
        if match_test:
          current_test = match_test.group(1)
        match_section = section_pattern.match(line)
        if match_section:
          current_test = match_section.group(1)
        for pattern in seed_patterns:
          match_seed = pattern.search(line)
          if match_seed and current_test:
            seed_map[current_test] = match_seed.group(1)
  except OSError as exc:
    logging.error("Failed to read log file %s: %s", log_path, exc)
  return seed_map


def select_seed(test_name: str, seeds: Dict[str, str]) -> Optional[str]:
  best_match: Optional[Tuple[int, str]] = None
  for candidate, seed in seeds.items():
    if candidate in test_name or test_name in candidate:
      score = len(candidate)
      if best_match is None or score > best_match[0]:
        best_match = (score, seed)
  if best_match:
    return best_match[1]
  return None


def attach_seeds(failures: List[FailureRecord], seeds: Dict[str, str]) -> None:
  for failure in failures:
    seed = select_seed(failure.name, seeds)
    if seed is None and failure.suite:
      seed = select_seed(failure.suite, seeds)
    if seed:
      failure.seed = seed


def compute_counts(failures: Sequence[FailureRecord]) -> Dict[str, int]:
  counts = {"P1": 0, "P2": 0, "P3": 0}
  for failure in failures:
    key = failure.severity if failure.severity in counts else DEFAULT_SEVERITY
    counts[key] += 1
  return counts


def decide_gate(counts: Dict[str, int]) -> str:
  if counts.get("P1", 0) > 0 or counts.get("P2", 0) > 0:
    return "fail"
  return "pass"


def build_json(
  gate_status: str,
  counts: Dict[str, int],
  failures: Sequence[FailureRecord],
) -> Dict[str, object]:
  failed_entries: List[Dict[str, object]] = []
  for failure in failures:
    entry: Dict[str, object] = {
      "suite": failure.suite,
      "name": failure.name,
      "severity": failure.severity,
      "time": failure.time,
      "message": failure.message,
    }
    if failure.seed:
      entry["seed"] = failure.seed
    failed_entries.append(entry)
  meta = {
    "sha": os.getenv("GITHUB_SHA", ""),
    "ref": os.getenv("GITHUB_REF", ""),
    "run_id": os.getenv("GITHUB_RUN_ID", ""),
  }
  return {
    "gate_status": gate_status,
    "counts": counts,
    "failed": failed_entries,
    "meta": meta,
  }


def format_table_row(failure: FailureRecord) -> str:
  message_clean = normalize_whitespace(failure.message)
  message_truncated = truncate_message(message_clean)
  time_display = f"{failure.time:.3f}" if failure.time else "0.000"
  seed_display = failure.seed or "-"
  return (
    f"| {failure.severity} | {failure.suite or '-'} | {failure.name} | "
    f"{time_display} | {seed_display} | {message_truncated} |"
  )


def build_markdown(
  gate_status: str,
  counts: Dict[str, int],
  failures: Sequence[FailureRecord],
) -> str:
  lines: List[str] = []
  lines.append("# A110 Gate Summary")
  lines.append("![badge](BADGE_PLACEHOLDER)")
  lines.append("")
  lines.append("| Severity | Suite | Test | Time | Seed | Message |")
  lines.append("| --- | --- | --- | --- | --- | --- |")
  if failures:
    for failure in failures:
      lines.append(format_table_row(failure))
  else:
    lines.append("| ✅ | - | Nenhuma falha | 0.000 | - | - |")
  lines.append("")
  lines.append("## Como reproduzir localmente")
  lines.append("```")
  if failures:
    for failure in failures:
      command = f"cargo test -- {failure.name}"
      if failure.seed:
        command = f"PROPTEST_SEED={failure.seed} {command}"
      lines.append(command)
  else:
    lines.append("cargo test")
  lines.append("```")
  lines.append("")
  lines.append(
    f"- Contagens: P1={counts['P1']}, P2={counts['P2']}, P3={counts['P3']}"
  )
  decision_icon = "✅" if gate_status == "pass" else "❌"
  decision_text = "Pass" if gate_status == "pass" else "Fail"
  lines.append(f"- Decisão: {decision_icon} {decision_text}")
  return "\n".join(lines) + "\n"


def write_json(path: Path, payload: Dict[str, object]) -> None:
  ensure_parent(path)
  with path.open("w", encoding="utf-8") as handle:
    json.dump(payload, handle, indent=2, sort_keys=True)
    handle.write("\n")


def write_markdown(path: Path, content: str) -> None:
  ensure_parent(path)
  with path.open("w", encoding="utf-8") as handle:
    handle.write(content)


def main(argv: Optional[Sequence[str]] = None) -> int:
  logging.basicConfig(level=logging.INFO, format="%(levelname)s: %(message)s")
  args = parse_arguments(argv)
  failures, parsed_ok = parse_junit(args.junit)
  seeds: Dict[str, str] = {}
  if args.log:
    seeds = parse_proptest_log(args.log)
  if failures and seeds:
    attach_seeds(failures, seeds)
  failures.sort()
  counts = compute_counts(failures)
  gate_status = decide_gate(counts)
  payload = build_json(gate_status, counts, failures)
  markdown = build_markdown(gate_status, counts, failures)
  try:
    write_json(args.summary_json, payload)
  except OSError as exc:
    logging.error("Failed to write JSON summary %s: %s", args.summary_json, exc)
  try:
    write_markdown(args.summary_md, markdown)
  except OSError as exc:
    logging.error("Failed to write Markdown summary %s: %s", args.summary_md, exc)
  if not parsed_ok:
    gate_status = "fail"
  return 0


if __name__ == "__main__":
  sys.exit(main())
