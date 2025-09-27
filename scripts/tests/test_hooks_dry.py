import importlib.util
import json
import pathlib

MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "hooks_dry.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("hooks_dry", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("Unable to load hooks_dry module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


hooks_dry = _load_module()


def _write_json(path: pathlib.Path, payload: object) -> None:
    path.write_text(json.dumps(payload), encoding="utf-8")


def test_load_watchers_combines_core_and_domain(tmp_path, monkeypatch):
    watchers_dir = tmp_path / "ops" / "watchers"
    watchers_dir.mkdir(parents=True)

    _write_json(
        watchers_dir / "core.yaml",
        {
            "domains": {
                "DEC": ["metrics_decision_hook_gap_watch", "model_drift_watch"],
            }
        },
    )

    _write_json(
        watchers_dir / "dec.yml",
        {
            "domain": "DEC",
            "watchers": [
                {"name": "metrics_decision_hook_gap_watch"},
                {"name": "model_drift_watch"},
            ],
        },
    )

    hooks_dir = tmp_path / "ops" / "hooks"
    hooks_dir.mkdir(parents=True)
    _write_json(
        hooks_dir / "a110.yml",
        [
            {
                "hook": "dec-latency-degrade",
                "domain": "DEC",
                "watchers": [
                    "metrics_decision_hook_gap_watch",
                    "model_drift_watch",
                ],
                "kpi": "dec.latency.p95",
                "threshold": "800ms",
                "window": "5m",
                "action": "degrade_route",
                "owner": "dec-duty@trendmarket",
                "rollback": "yes",
            }
        ],
    )

    monkeypatch.chdir(tmp_path)

    watchers, errors = hooks_dry._load_watchers()
    assert errors == []
    assert watchers == {
        "DEC": {"metrics_decision_hook_gap_watch", "model_drift_watch"}
    }

    assert hooks_dry.main() == 0


def test_hooks_dry_fails_when_core_and_domain_watchers_diverge(tmp_path, monkeypatch):
    watchers_dir = tmp_path / "ops" / "watchers"
    watchers_dir.mkdir(parents=True)

    _write_json(
        watchers_dir / "core.yaml",
        {
            "domains": {
                "DEC": ["metrics_decision_hook_gap_watch", "model_drift_watch"],
            }
        },
    )

    _write_json(
        watchers_dir / "dec.yml",
        {
            "domain": "DEC",
            "watchers": [
                {"name": "metrics_decision_hook_gap_watch"},
                {"name": "slo_budget_breach_watch"},
            ],
        },
    )

    monkeypatch.chdir(tmp_path)

    watchers, errors = hooks_dry._load_watchers()
    assert "DEC" in watchers
    assert any("watcher mismatch for domain 'DEC'" in message for message in errors)

    # hooks_dry.main() should abort before reading hooks due to the mismatch
    assert hooks_dry.main() == 1
