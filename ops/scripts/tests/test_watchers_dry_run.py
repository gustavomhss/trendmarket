import importlib.util
import json
import pathlib

import pytest


MODULE_PATH = pathlib.Path(__file__).resolve().parents[1] / "watchers_dry_run.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("watchers_dry_run", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("Unable to load watchers_dry_run module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


watchers_dry_run = _load_module()


def _build_watcher(**overrides):
    watcher = {
        "id": "sample",
        "domain": "DEC",
        "owner": "owner",
        "kpi": "metric",
        "threshold": "1",
        "window": "5m",
        "action": "noop",
        "hook_id": "dec-latency-degrade",
    }
    watcher.update(overrides)
    return watcher


def _write_core(path: pathlib.Path, domain: str, watcher_ids: list[str], hook: str = "dec-latency-degrade"):
    payload = {
        "version": 2,
        "domains": {domain: watcher_ids},
        "watchers": {
            watcher_id: {
                "description": "",
                "kpi": "",
                "owner": "",
                "hooks": {domain: hook},
            }
            for watcher_id in watcher_ids
        },
    }
    path.write_text(json.dumps(payload), encoding="utf-8")


def test_validate_watcher_requires_domain():
    watcher = _build_watcher()
    watcher.pop("domain")

    with pytest.raises(ValueError) as excinfo:
        watchers_dry_run._validate_watcher(watcher)

    message = str(excinfo.value)
    assert "missing fields" in message
    assert "'domain'" in message


def test_validate_watcher_normalizes_domain(tmp_path):
    watcher = _build_watcher(domain="  DEC  ")
    config_path = tmp_path / "watchers.json"
    config = {"domain": "  DEC  ", "watchers": [watcher]}
    config_path.write_text(json.dumps(config), encoding="utf-8")
    _write_core(tmp_path / "core.yaml", "DEC", ["sample"])

    report = watchers_dry_run.generate_report(config_path)

    assert report["watchers"][0]["domain"] == "DEC"


def test_generate_report_from_directory(tmp_path):
    watchers_dir = tmp_path / "watchers"
    watchers_dir.mkdir()
    watcher_file = watchers_dir / "dec.yml"
    watcher_config = {
        "domain": "DEC",
        "watchers": [
            {
                "name": "model_drift_watch",
                "owner": "ml-ops@trendmarket",
                "kpi": "ml.model.psi",
                "threshold": ">0.2",
                "window": "24h",
                "action": "rollback_model",
                "rollback": "yes",
            }
        ],
    }
    watcher_file.write_text(json.dumps(watcher_config), encoding="utf-8")
    _write_core(watchers_dir / "core.yaml", "DEC", ["model_drift_watch"])

    report = watchers_dry_run.generate_report(watchers_dir)

    assert report["total_watchers"] == 1
    watcher = report["watchers"][0]
    assert watcher["id"] == "model_drift_watch"
    assert watcher["domain"] == "DEC"
    assert watcher["rollback"] == "yes"
    assert watcher["hook_id"] == "dec-latency-degrade"
