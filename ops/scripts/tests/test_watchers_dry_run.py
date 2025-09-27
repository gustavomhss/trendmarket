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


def _write_core_config(tmp_path, payload):
    config_path = tmp_path / "core.yaml"
    config_path.write_text(json.dumps(payload), encoding="utf-8")
    return config_path


def test_generate_report_from_core_config(tmp_path):
    config_path = _write_core_config(
        tmp_path,
        {
            "version": 1,
            "domains": {"  DEC  ": ["  model_drift_watch  "]},
            "watchers": {
                "model_drift_watch": {
                    "owner": "ML",
                    "description": "Detecta drift",
                    "kpi": "ml.model.psi",
                }
            },
        },
    )

    report = watchers_dry_run.generate_report(config_path)

    assert report["total_watchers"] == 1
    watcher = report["watchers"][0]
    assert watcher["id"] == "model_drift_watch"
    assert watcher["domain"] == "DEC"
    assert watcher["domains"] == ["DEC"]
    assert watcher["owner"] == "ML"
    assert watcher["description"] == "Detecta drift"
    assert watcher["kpi"] == "ml.model.psi"
    assert "hash" in watcher


def test_generate_report_requires_metadata(tmp_path):
    config_path = _write_core_config(
        tmp_path,
        {
            "version": 1,
            "domains": {"DEC": ["model_drift_watch"]},
            "watchers": {"model_drift_watch": {"description": "missing owner"}},
        },
    )

    with pytest.raises(ValueError) as excinfo:
        watchers_dry_run.generate_report(config_path)

    assert "missing required field 'owner'" in str(excinfo.value)


def test_generate_report_requires_domain_assignment(tmp_path):
    config_path = _write_core_config(
        tmp_path,
        {
            "version": 1,
            "domains": {"DEC": ["model_drift_watch"]},
            "watchers": {
                "model_drift_watch": {"owner": "ML", "description": "desc"},
                "orphan_watch": {"owner": "ML", "description": "desc"},
            },
        },
    )

    with pytest.raises(ValueError) as excinfo:
        watchers_dry_run.generate_report(config_path)

    assert "watchers missing domain assignment: orphan_watch" in str(excinfo.value)
