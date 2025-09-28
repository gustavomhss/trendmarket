#!/usr/bin/env python3
"""Aggregate dry-run artefacts into an evidence bundle."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any, Callable, Dict


def _load_yaml_parser() -> Callable[[str], Any] | None:
    spec = importlib.util.find_spec("yaml")
    if spec is None:
        return None

    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    spec.loader.exec_module(module)  # type: ignore[attr-defined]
    return module.safe_load  # type: ignore[return-value]


_YAML_PARSER = _load_yaml_parser()


def _parse_content(text: str) -> Any:
    yaml_error: Exception | None = None
    if _YAML_PARSER is not None:
        try:
            return _YAML_PARSER(text)
        except Exception as exc:  # pragma: no cover - safe fallback
            yaml_error = exc

    try:
        return json.loads(text)
    except json.JSONDecodeError as exc:
        if yaml_error is not None:
            raise RuntimeError(
                f"Unable to parse artefact as YAML ({yaml_error}) or JSON"
            ) from exc
        raise RuntimeError(f"Invalid JSON: {exc}") from exc


def _load_document(path: pathlib.Path) -> Dict[str, Any]:
    if not path.exists():
        raise FileNotFoundError(f"Required artefact not found: {path}")
    payload = _parse_content(path.read_text(encoding="utf-8"))
    if not isinstance(payload, dict):
        raise RuntimeError(f"Artefact at {path} must be a mapping")
    return payload


def _digest(content: Dict[str, Any]) -> str:
    encoded = json.dumps(content, sort_keys=True, ensure_ascii=False).encode("utf-8")
    return hashlib.sha256(encoded).hexdigest()


def aggregate(watchers_path: pathlib.Path, hooks_path: pathlib.Path) -> Dict[str, Any]:
    watchers = _load_document(watchers_path)
    hooks = _load_document(hooks_path)
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
