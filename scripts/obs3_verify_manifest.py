"""Validate OBS-3 evidence manifests against the canonical schema."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any, Iterable

from jsonschema import Draft7Validator, SchemaError


DEFAULT_MANIFEST = "out/obs_gatecheck/evidence/prom_scrape.json"
DEFAULT_SCHEMA = "ops/schemas/manifest.schema.json"


def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description=(
            "Validate an OBS-3 evidence manifest against the canonical schema."
        )
    )
    parser.add_argument(
        "--manifest",
        default=DEFAULT_MANIFEST,
        help=f"Path to the manifest JSON file (default: {DEFAULT_MANIFEST}).",
    )
    parser.add_argument(
        "--schema",
        default=DEFAULT_SCHEMA,
        help=f"Path to the JSON schema file (default: {DEFAULT_SCHEMA}).",
    )
    parser.add_argument(
        "--pretty",
        action="store_true",
        help="Accepted for compatibility; output remains plain text.",
    )
    parser.add_argument(
        "--strict",
        action="store_true",
        help="Reserved for future use; currently a no-op.",
    )
    return parser.parse_args()


def load_json(path: Path) -> Any:
    """Load JSON content from a file."""
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def format_error_path(path: Iterable[Any]) -> str:
    """Convert a jsonschema error path into a slash separated string."""
    parts = [str(component) for component in path]
    return "/".join(parts) if parts else "(root)"


def validate_manifest(manifest: Any, schema: Any) -> list[str]:
    """Validate the manifest payload against the provided schema."""
    validator = Draft7Validator(schema)
    errors = sorted(validator.iter_errors(manifest), key=lambda err: err.path)
    messages: list[str] = []
    for error in errors[:5]:
        location = format_error_path(error.path)
        messages.append(f"{location}: {error.message}")
    if len(errors) > 5:
        messages.append(f"... and {len(errors) - 5} more errors")
    return messages


def main() -> int:
    """Entry point for CLI execution."""
    args = parse_args()
    schema_path = Path(args.schema)
    manifest_path = Path(args.manifest)

    try:
        schema = load_json(schema_path)
    except FileNotFoundError:
        print(f"schema not found: {schema_path}", file=sys.stderr)
        return 4
    except OSError as exc:
        print(f"schema inaccessible: {schema_path}: {exc}", file=sys.stderr)
        return 4
    except json.JSONDecodeError as exc:
        print(f"invalid schema json: {schema_path}: {exc}", file=sys.stderr)
        return 6
    try:
        manifest = load_json(manifest_path)
    except FileNotFoundError:
        print(f"manifest not found: {manifest_path}", file=sys.stderr)
        return 5
    except OSError as exc:
        print(f"manifest inaccessible: {manifest_path}: {exc}", file=sys.stderr)
        return 5
    except json.JSONDecodeError as exc:
        print(f"invalid manifest json: {manifest_path}: {exc}", file=sys.stderr)
        return 6

    try:
        errors = validate_manifest(manifest, schema)
    except SchemaError as exc:
        print(f"invalid schema definition: {exc.message}", file=sys.stderr)
        return 6
    except Exception as exc:  # pragma: no cover - defensive guardrail
        print(f"unexpected validation error: {exc}", file=sys.stderr)
        return 9

    if errors:
        print("manifest invalid", file=sys.stderr)
        for message in errors:
            print(f"  - {message}", file=sys.stderr)
        return 7

    print("manifest valid")
    return 0


if __name__ == "__main__":
    sys.exit(main())
