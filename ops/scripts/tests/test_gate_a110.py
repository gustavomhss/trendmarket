import pathlib
import shutil
import subprocess

import pytest


REPO_ROOT = pathlib.Path(__file__).resolve().parents[3]
SCRIPT_PATH = REPO_ROOT / "ops" / "scripts" / "gate_a110.sh"
HOOKS_PATH = REPO_ROOT / "ops" / "hooks" / "a110.yml"


def _run_gate() -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(SCRIPT_PATH), "--require-green"],
        cwd=REPO_ROOT,
        capture_output=True,
        text=True,
        check=False,
    )


@pytest.fixture()
def _restore_hooks():
    original_bytes = HOOKS_PATH.read_bytes()
    try:
        yield
    finally:
        HOOKS_PATH.write_bytes(original_bytes)


def test_gate_fails_when_hooks_manifest_missing(tmp_path, _restore_hooks):
    backup_path = tmp_path / "a110.yml.bak"
    shutil.move(HOOKS_PATH, backup_path)
    try:
        result = _run_gate()
    finally:
        shutil.move(backup_path, HOOKS_PATH)

    combined_output = "\n".join(part for part in (result.stdout, result.stderr) if part)
    assert result.returncode != 0, combined_output
    assert "missing hooks file" in combined_output


def test_gate_fails_when_hooks_manifest_malformed(_restore_hooks):
    HOOKS_PATH.write_text("{ not valid json", encoding="utf-8")

    result = _run_gate()

    combined_output = "\n".join(part for part in (result.stdout, result.stderr) if part)
    assert result.returncode != 0, combined_output
    assert "failed to parse" in combined_output
