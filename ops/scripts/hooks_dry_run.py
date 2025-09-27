#!/usr/bin/env python3
"""Validate hook specifications for A110 dry-runs."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import sys
from datetime import datetime, timezone
from typing import Any, Dict, List

REQUIRED_FIELDS = {"hook", "kpi", "threshold", "window", "action", "owner", "rollback"}


def _load_hooks(path: pathlib.Path) -> List[Dict[str, Any]]:
    if not path.exists():
        raise FileNotFoundError(f"Hook configuration not found: {path}")
    data = json.loads(path.read_text(encoding="utf-8"))
    hooks = data.get("hooks") if isinstance(data, dict) else None
    if not isinstance(hooks, list):
        raise ValueError("Hook configuration must contain a 'hooks' list")
    return hooks


def _normalize(value: Any) -> Any:
    if isinstance(value, str):
        return value.strip()
    return value


def _coerce_rollback(value: Any) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized == "yes":
            return True
        if normalized == "no":
            return False
    raise ValueError(
        "Rollback flag must be a boolean or the string 'yes'/'no'"
    )


def _validate_hook(hook: Dict[str, Any]) -> Dict[str, Any]:
    missing = REQUIRED_FIELDS - hook.keys()
    if missing:
        raise ValueError(f"Hook '{hook.get('hook')}' missing fields: {sorted(missing)}")
    normalized = {key: _normalize(hook[key]) for key in sorted(REQUIRED_FIELDS)}
    normalized["rollback"] = _coerce_rollback(normalized["rollback"])
    encoded = json.dumps(normalized, sort_keys=True, ensure_ascii=False).encode("utf-8")
    digest = hashlib.sha256(encoded).hexdigest()
    return {
        "hook": normalized["hook"],
        "kpi": normalized["kpi"],
        "threshold": normalized["threshold"],
        "window": normalized["window"],
        "action": normalized["action"],
        "owner": normalized["owner"],
        "rollback": bool(normalized["rollback"]),
        "evidence": hook.get("evidence", {}),
        "hash": digest,
    }


def generate_report(config: pathlib.Path) -> Dict[str, Any]:
    hooks = _load_hooks(config)
    validated = [_validate_hook(h) for h in hooks]
    return {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "source": str(config),
        "total_hooks": len(validated),
        "hooks": validated,
    }


def main(argv: List[str]) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args(argv)

    report = generate_report(args.config)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
