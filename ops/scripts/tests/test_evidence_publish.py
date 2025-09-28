import importlib.util
import json
import pathlib

import pytest

MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "evidence_publish.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("evidence_publish", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("Unable to load evidence_publish module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


evidence_publish = _load_module()


def _write_json(path: pathlib.Path, payload: dict[str, object]) -> pathlib.Path:
    path.write_text(json.dumps(payload), encoding="utf-8")
    return path


def test_aggregate_supports_json_inputs(tmp_path):
    watchers_path = _write_json(
        tmp_path / "watchers.json",
        {
            "total_watchers": 1,
            "watchers": [{"id": "model_drift_watch"}],
        },
    )
    hooks_path = _write_json(
        tmp_path / "hooks.json",
        {
            "total_hooks": 1,
            "hooks": [{"id": "dec-latency-degrade"}],
        },
    )

    bundle = evidence_publish.aggregate(watchers_path, hooks_path)

    assert bundle["watchers_count"] == 1
    assert bundle["hooks_count"] == 1
    assert bundle["evidence"]["watchers"][0]["id"] == "model_drift_watch"
    assert bundle["evidence"]["hooks"][0]["id"] == "dec-latency-degrade"


def test_aggregate_supports_yaml_inputs(tmp_path):
    pytest.importorskip("yaml")

    watchers_path = tmp_path / "watchers.yaml"
    watchers_path.write_text(
        """
        total_watchers: 2
        watchers:
          - id: metrics_decision_hook_gap_watch
          - id: slo_budget_breach_watch
        """.strip(),
        encoding="utf-8",
    )

    hooks_path = tmp_path / "hooks.yaml"
    hooks_path.write_text(
        """
        total_hooks: 2
        hooks:
          - id: dec-latency-degrade
          - id: slo-burn-rate-guard
        """.strip(),
        encoding="utf-8",
    )

    bundle = evidence_publish.aggregate(watchers_path, hooks_path)

    assert bundle["watchers_count"] == 2
    assert bundle["hooks_count"] == 2
    watcher_ids = {entry["id"] for entry in bundle["evidence"]["watchers"]}
    hook_ids = {entry["id"] for entry in bundle["evidence"]["hooks"]}
    assert watcher_ids == {
        "metrics_decision_hook_gap_watch",
        "slo_budget_breach_watch",
    }
    assert hook_ids == {
        "dec-latency-degrade",
        "slo-burn-rate-guard",
    }
