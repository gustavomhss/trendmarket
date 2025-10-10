#!/usr/bin/env python3
"""Validate OBS-3 manifest against the canonical schema."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

import jsonschema


def read_json(path: Path):
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def main() -> int:
    parser = argparse.ArgumentParser(description="Verify OBS-3 evidence manifest")
    parser.add_argument("--schema", required=True, type=Path)
    parser.add_argument("--manifest", required=True, type=Path)
    args = parser.parse_args()

    schema = read_json(args.schema)
    manifest = read_json(args.manifest)

    validator = jsonschema.Draft7Validator(schema)
    errors = sorted(validator.iter_errors(manifest), key=lambda e: e.path)
    if errors:
        for error in errors:
            path = ".".join(str(x) for x in error.absolute_path)
            print(f"Schema violation at {path or '<root>'}: {error.message}")
        return 1

    print(f"Manifest {args.manifest} is valid against {args.schema}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
