"""OBS-3 Thread 6: hash evidence files and update manifest metadata."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from subprocess import CalledProcessError, CompletedProcess, run
from tempfile import NamedTemporaryFile
from typing import Dict, Iterable, List, Mapping, MutableMapping, Optional
from uuid import uuid4


EXIT_NO_EVIDENCE = 5
EXIT_IO_ERROR = 13
EXIT_JSON_ERROR = 14
EXIT_HASH_ERROR = 15


def parse_args(argv: Optional[Iterable[str]] = None) -> argparse.Namespace:
    """Parse command line arguments."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--evidence-dir",
        default="out/obs_gatecheck/evidence",
        help="Directory containing evidence JSON files.",
    )
    parser.add_argument(
        "--manifest",
        default=None,
        help="Manifest file to update (default: <evidence-dir>/prom_scrape.json).",
    )
    parser.add_argument(
        "--spec-version",
        default="5.0",
        help="Specification version to record.",
    )
    parser.add_argument(
        "--git-sha",
        default=None,
        help="Git SHA override; if omitted, detect via git.",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Print the resulting manifest to stdout without writing to disk.",
    )
    parser.add_argument(
        "--pretty",
        action="store_true",
        help="Pretty-print JSON with indentation when writing or in dry-run mode.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Emit verbose progress messages to stderr.",
    )
    return parser.parse_args(argv)


def log(message: str, *, verbose: bool) -> None:
    """Write a message to stderr when verbose mode is active."""

    if verbose:
        print(message, file=sys.stderr)


def list_evidence_files(evidence_dir: Path, manifest_path: Path, *, verbose: bool) -> List[Path]:
    """Return sorted evidence files excluding the manifest itself."""

    try:
        entries = sorted(p for p in evidence_dir.iterdir() if p.suffix == ".json")
    except FileNotFoundError:
        print(f"evidence directory not found: {evidence_dir}", file=sys.stderr)
        sys.exit(EXIT_NO_EVIDENCE)
    except OSError as exc:  # pragma: no cover - defensive
        print(f"failed to read evidence directory: {exc}", file=sys.stderr)
        sys.exit(EXIT_IO_ERROR)

    manifest_resolved = manifest_path.resolve()
    evidence_files: List[Path] = []
    for entry in entries:
        if entry.resolve() == manifest_resolved:
            log(f"skipping manifest file: {entry.name}", verbose=verbose)
            continue
        evidence_files.append(entry)
    return evidence_files


def sha256_file(path: Path) -> str:
    """Compute the SHA256 hash of a file."""

    digest = hashlib.sha256()
    try:
        with path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(8192), b""):
                digest.update(chunk)
    except OSError as exc:
        print(f"failed to hash {path}: {exc}", file=sys.stderr)
        sys.exit(EXIT_HASH_ERROR)
    return digest.hexdigest()


def detect_git_sha(override: Optional[str], *, verbose: bool) -> str:
    """Return the git SHA, using override or discovering via git."""

    if override:
        log(f"using provided git SHA: {override}", verbose=verbose)
        return override

    try:
        result: CompletedProcess[str] = run(
            ["git", "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
        )
    except (CalledProcessError, FileNotFoundError):
        log("failed to detect git SHA; using 'unknown'", verbose=verbose)
        return "unknown"

    sha = result.stdout.strip()
    if not sha:
        log("empty git SHA detected; using 'unknown'", verbose=verbose)
        return "unknown"
    return sha


def load_manifest_safely(path: Path, *, verbose: bool) -> MutableMapping[str, object]:
    """Load existing manifest JSON if present."""

    if not path.exists():
        log(f"manifest does not exist; will create new file at {path}", verbose=verbose)
        return {}

    try:
        with path.open("r", encoding="utf-8") as handle:
            data = json.load(handle)
    except json.JSONDecodeError as exc:
        print(f"invalid JSON in manifest {path}: {exc}", file=sys.stderr)
        sys.exit(EXIT_JSON_ERROR)
    except OSError as exc:
        print(f"failed to read manifest {path}: {exc}", file=sys.stderr)
        sys.exit(EXIT_IO_ERROR)

    if not isinstance(data, MutableMapping):
        print(f"manifest {path} must be a JSON object", file=sys.stderr)
        sys.exit(EXIT_JSON_ERROR)
    return data


def atomic_write_json(path: Path, payload: Mapping[str, object], *, pretty: bool) -> None:
    """Write JSON to path atomically."""

    json_kwargs = {"ensure_ascii": False, "sort_keys": True}
    if pretty:
        json_kwargs.update({"indent": 2})
    else:
        json_kwargs.update({"separators": (",", ":")})

    directory = path.parent
    os.makedirs(directory, exist_ok=True)

    tmp_file: Optional[str] = None
    try:
        with NamedTemporaryFile(
            "w", encoding="utf-8", dir=directory, delete=False, prefix=path.name, suffix=".tmp"
        ) as handle:
            tmp_file = handle.name
            json.dump(payload, handle, **json_kwargs)
            handle.write("\n")
        os.chmod(tmp_file, 0o644)
        os.replace(tmp_file, path)
    except OSError as exc:
        if tmp_file and os.path.exists(tmp_file):
            os.unlink(tmp_file)
        print(f"failed to write manifest {path}: {exc}", file=sys.stderr)
        sys.exit(EXIT_IO_ERROR)


def build_integrity_map(files: Iterable[Path], evidence_dir: Path, *, verbose: bool) -> Dict[str, str]:
    """Compute the integrity mapping for evidence files."""

    entries: Dict[str, str] = {}
    for file_path in sorted(files, key=lambda p: p.name):
        relative: str
        try:
            relative = str(file_path.relative_to(evidence_dir))
        except ValueError:
            relative = file_path.name
        digest = sha256_file(file_path)
        entries[relative] = digest
        log(f"hashed {relative}: {digest}", verbose=verbose)
    return entries


def build_manifest_update(
    existing: MutableMapping[str, object],
    integrity: Mapping[str, str],
    spec_version: str,
    git_sha: str,
) -> Dict[str, object]:
    """Return a new manifest dictionary containing merged data."""

    updated: Dict[str, object] = dict(existing)
    updated["run_id"] = str(uuid4())
    updated["spec_version"] = spec_version
    updated["git_sha"] = git_sha
    timestamp = datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z")
    updated["ts"] = timestamp
    updated["integrity"] = dict(integrity)
    return updated


def main(argv: Optional[Iterable[str]] = None) -> int:
    """Run the CLI entrypoint."""

    args = parse_args(argv)
    evidence_dir = Path(args.evidence_dir).resolve()
    manifest_path = Path(args.manifest) if args.manifest else evidence_dir / "prom_scrape.json"

    log(f"evidence directory: {evidence_dir}", verbose=args.verbose)
    log(f"manifest path: {manifest_path}", verbose=args.verbose)

    evidence_files = list_evidence_files(evidence_dir, manifest_path, verbose=args.verbose)
    if not evidence_files:
        print("no evidence files", file=sys.stderr)
        return EXIT_NO_EVIDENCE

    integrity = build_integrity_map(evidence_files, evidence_dir, verbose=args.verbose)
    git_sha = detect_git_sha(args.git_sha, verbose=args.verbose)
    existing_manifest = load_manifest_safely(manifest_path, verbose=args.verbose)
    updated_manifest = build_manifest_update(existing_manifest, integrity, args.spec_version, git_sha)

    if args.dry_run:
        json_kwargs = {"ensure_ascii": False, "sort_keys": True}
        if args.pretty:
            json_kwargs.update({"indent": 2})
        else:
            json_kwargs.update({"separators": (",", ":")})
        json.dump(updated_manifest, sys.stdout, **json_kwargs)
        sys.stdout.write("\n")
        return 0

    atomic_write_json(manifest_path, updated_manifest, pretty=args.pretty)
    log(f"manifest updated at {manifest_path}", verbose=args.verbose)
    return 0


if __name__ == "__main__":  # pragma: no cover - CLI entrypoint
    sys.exit(main())
