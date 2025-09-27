"""Tests for the hooks dry-run validation helpers."""

from __future__ import annotations

import importlib.util
import json
import pathlib

import pytest


MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "hooks_dry_run.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("hooks_dry_run", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("Unable to load hooks_dry_run module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


hooks_dry_run = _load_module()


def _build_hook(**overrides):
    hook = {
        "hook": "sample",
        "kpi": "metric",
        "threshold": "1",
        "window": "5m",
        "action": "noop",
        "owner": "SRE",
        "rollback": "yes",
    }
    hook.update(overrides)
    return hook


@pytest.fixture
def list_config_path(tmp_path: pathlib.Path) -> pathlib.Path:
    config_path = tmp_path / "hooks.yml"
    hooks = [
        _build_hook(hook="list-alpha", rollback=True),
        _build_hook(hook="list-beta", rollback="no"),
    ]
    config_path.write_text(json.dumps(hooks), encoding="utf-8")
    return config_path


def test_validate_hook_accepts_string_no():
    hook = _build_hook(rollback="no")
    validated = hooks_dry_run._validate_hook(hook)
    assert validated["rollback"] is False


def test_validate_hook_accepts_string_yes_case_insensitive():
    hook = _build_hook(rollback="YeS")
    validated = hooks_dry_run._validate_hook(hook)
    assert validated["rollback"] is True


def test_validate_hook_rejects_unexpected_string():
    hook = _build_hook(rollback="maybe")
    with pytest.raises(ValueError):
        hooks_dry_run._validate_hook(hook)


def test_generate_report_emits_boolean_for_string_no(tmp_path):
    config_path = tmp_path / "hooks.json"
    config = {"hooks": [_build_hook(rollback="no")]}
    config_path.write_text(json.dumps(config), encoding="utf-8")

    report = hooks_dry_run.generate_report(config_path)

    assert report["hooks"][0]["rollback"] is False


def test_generate_report_supports_top_level_list(list_config_path: pathlib.Path):
    report = hooks_dry_run.generate_report(list_config_path)

    assert report["total_hooks"] == 2
    assert {entry["hook"] for entry in report["hooks"]} == {"list-alpha", "list-beta"}
