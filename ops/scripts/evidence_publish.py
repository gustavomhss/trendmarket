#!/usr/bin/env python3
"""Aggregate dry-run artefacts into an evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any, Dict


def _load_json(path: pathlib.Path) -> Dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(f"Required artefact not found: {path}")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError as exc:
        raise RuntimeError(f"Invalid JSON at {path}: {exc}") from exc


def _digest(content: Dict[str, Any]) -> str:
    encoded = json.dumps(content, sort_keys=True, ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def aggregate(watchers_path: pathlib.Path, hooks_path: pathlib.Path) -> Dict[str, Any]:
    watchers = _load_json(watchers_path)
    hooks = _load_json(hooks_path)
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "watchers_artifact": str(watchers_path),
        "hooks_artifact": str(hooks_path),
        "watchers_digest": _digest(watchers),
        "hooks_digest": _digest(hooks),
        "watchers_count": watchers.get("total_watchers", 0),
        "hooks_count": hooks.get("total_hooks", 0),
        "evidence": {
            "watchers": watchers.get("watchers", []),
            "hooks": hooks.get("hooks", []),
        },
    }


def main(argv: list[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--watchers", required=True, type=pathlib.Path)
    parser.add_argument("--hooks", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args(argv)

    bundle = aggregate(args.watchers, args.hooks)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(bundle, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
