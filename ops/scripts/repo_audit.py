#!/usr/bin/env python3
"""Append command executions to the repository audit log."""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import List


def load_entries(path: pathlib.Path) -> List[dict]:
    if not path.exists():
        return []
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"Invalid audit log at {path}: {exc}") from exc
    if not isinstance(data, list):
        raise RuntimeError(f"Audit log must contain a list, found {type(data).__name__}")
    return data


def append_entry(path: pathlib.Path, command: str, status: str, artifacts: List[str]) -> None:
    entries = load_entries(path)
    entries.append(
        {
            "command": command,
            "status": status,
            "artifacts": artifacts,
            "recorded_at": datetime.now(timezone.utc).isoformat(),
        }
    )
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(entries, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log", required=True, type=pathlib.Path)
    parser.add_argument("--command", required=True)
    parser.add_argument("--status", required=True)
    parser.add_argument("--artifact", action="append", default=[])
    args = parser.parse_args(argv)

    append_entry(args.log, args.command, args.status, args.artifact)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
