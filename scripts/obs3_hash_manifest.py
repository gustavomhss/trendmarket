#!/usr/bin/env python3
"""Generate OBS-3 manifest with SHA256 hashes for evidence files."""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SPEC_VERSION = "5.0"


def sha256sum(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git_rev() -> str:
    try:
        return subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    except subprocess.CalledProcessError as exc:
        raise SystemExit(f"Unable to determine git revision: {exc}") from exc


def load_quality(evidence_dir: Path) -> dict[str, Any]:
    report_path = evidence_dir / "quality_report.json"
    if report_path.exists():
        with report_path.open("r", encoding="utf-8") as handle:
            return json.load(handle)
    return {}


def repo_root() -> Path:
    try:
        root = subprocess.check_output(["git", "rev-parse", "--show-toplevel"], text=True).strip()
    except subprocess.CalledProcessError as exc:
        raise SystemExit(f"Unable to determine repository root: {exc}") from exc
    return Path(root)


def build_artifacts(evidence_dir: Path, output_path: Path) -> list[dict[str, Any]]:
    artifacts: list[dict[str, Any]] = []
    root = repo_root()
    output_resolved = output_path.resolve()
    for file_path in sorted(evidence_dir.rglob("*")):
        if file_path.is_dir():
            continue
        if file_path.resolve() == output_resolved:
            continue
        try:
            rel_path = file_path.relative_to(root)
        except ValueError:
            rel_path = file_path
        artifacts.append(
            {
                "name": file_path.name,
                "path": str(rel_path),
                "sha256": sha256sum(file_path),
                "size_bytes": file_path.stat().st_size,
            }
        )
    return artifacts


def extract_quality_gates(quality_report: dict[str, Any]) -> dict[str, Any]:
    checks = quality_report.get("checks", {})
    cardinality = quality_report.get("cardinality_snapshot", {"total_series": 0, "by_service": {}})
    gates = {
        "histogram_monotonic": checks.get("histogram_monotonic", {}).get("ok", False),
        "closure_within_3pct": checks.get("closure_within_3pct", {}).get("ok", False),
        "quantile_ordering": checks.get("quantile_ordering", {}).get("ok", False),
        "counter_monotonic": checks.get("counter_monotonic", {}).get("ok", False),
        "nan_inf_zero": checks.get("nan_inf_zero", {}).get("ok", False),
        "cardinality_snapshot": cardinality,
    }
    return gates


def main() -> int:
    parser = argparse.ArgumentParser(description="Create OBS-3 manifest")
    parser.add_argument(
        "--env",
        required=True,
        choices=["dev", "prod"],
        help="Environment of the scrape",
    )
    parser.add_argument("--run-id", required=True, help="UUID for the evidence run")
    parser.add_argument(
        "--evidence-dir",
        required=True,
        type=Path,
        help="Directory containing evidence",
    )
    parser.add_argument(
        "--output",
        required=True,
        type=Path,
        help="Output manifest path",
    )
    args = parser.parse_args()

    evidence_dir = args.evidence_dir
    if not evidence_dir.exists():
        raise SystemExit(f"Evidence directory not found: {evidence_dir}")

    git_sha = git_rev()
    quality_report = load_quality(evidence_dir)
    artifacts = build_artifacts(evidence_dir, args.output)
    gates = extract_quality_gates(quality_report)

    manifest = {
        "spec_version": SPEC_VERSION,
        "run_id": args.run_id,
        "git_sha": git_sha,
        "ts": datetime.now(timezone.utc).isoformat(),
        "env": args.env,
        "artifacts": artifacts,
        "quality_gates": gates,
    }

    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("w", encoding="utf-8") as handle:
        json.dump(manifest, handle, indent=2, sort_keys=True)
        handle.write("\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
