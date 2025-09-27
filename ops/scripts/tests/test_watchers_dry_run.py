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


@pytest.fixture(autouse=True)
def reset_hook_metadata(monkeypatch):
    monkeypatch.setattr(watchers_dry_run, "_HOOK_METADATA_CACHE", None, raising=False)


def _build_watcher(**overrides):
    watcher = {
        "id": "sample",
        "domain": "DEC",
        "owner": "owner",
        "kpi": "metric",
        "threshold": "1",
        "window": "5m",
        "action": "noop",
    }
    watcher.update(overrides)
    return watcher


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

    report = watchers_dry_run.generate_report(watchers_dir)

    assert report["total_watchers"] == 1
    watcher = report["watchers"][0]
    assert watcher["id"] == "model_drift_watch"
    assert watcher["domain"] == "DEC"
    assert watcher["rollback"] == "yes"


def test_generate_report_from_aggregated_schema(tmp_path):
    config_path = tmp_path / "core.yaml"
    aggregated = {
        "version": 1,
        "domains": {
            "DEC": [
                "metrics_decision_hook_gap_watch",
                "slo_budget_breach_watch",
            ]
        },
        "watchers": {
            "metrics_decision_hook_gap_watch": {
                "description": "Latency guard",
                "kpi": "dec.latency.p95",
                "owner": "SRE",
                "hook": "dec-latency-gap",
            },
            "slo_budget_breach_watch": {
                "description": "Burn rate monitor",
                "kpi": "slo.burn_rate",
                "owner": "SRE",
                "hook": "slo-burn-rate-guard",
            },
        },
    }
    config_path.write_text(json.dumps(aggregated), encoding="utf-8")

    report = watchers_dry_run.generate_report(config_path)

    assert report["total_watchers"] == 2
    ids = {watcher["id"] for watcher in report["watchers"]}
    assert ids == {
        "metrics_decision_hook_gap_watch",
        "slo_budget_breach_watch",
    }

    by_id = {watcher["id"]: watcher for watcher in report["watchers"]}
    metric_gap = by_id["metrics_decision_hook_gap_watch"]
    assert metric_gap["domain"] == "DEC"
    assert metric_gap["action"] == "degrade_route"
    assert metric_gap["threshold"] == "800ms"
    assert metric_gap["window"] == "5m"
    assert metric_gap["description"] == "Latency guard"


def test_aggregated_schema_allows_domain_overrides(tmp_path, monkeypatch):
    config_path = tmp_path / "core.yaml"
    aggregated = {
        "version": 1,
        "domains": {
            "DEC": [
                {
                    "watcher": "metrics_decision_hook_gap_watch",
                    "id": "dec_metrics_gap_watch",
                    "owner": "dec-duty@trendmarket",
                    "threshold": "900ms",
                    "window": "1m",
                    "action": "page_dec_oncall",
                    "description": "DEC override",
                }
            ]
        },
        "watchers": {
            "metrics_decision_hook_gap_watch": {
                "description": "Default latency guard"
            }
        },
    }
    config_path.write_text(json.dumps(aggregated), encoding="utf-8")

    monkeypatch.setattr(
        watchers_dry_run,
        "_HOOK_METADATA_CACHE",
        {
            "metrics_decision_hook_gap_watch": {
                "owner": "SRE",
                "kpi": "dec.latency.p95",
                "threshold": "800ms",
                "window": "5m",
                "action": "degrade_route",
                "rollback": "yes",
            }
        },
        raising=False,
    )

    report = watchers_dry_run.generate_report(config_path)

    assert report["total_watchers"] == 1
    watcher = report["watchers"][0]
    assert watcher["id"] == "dec_metrics_gap_watch"
    assert watcher["domain"] == "DEC"
    assert watcher["owner"] == "dec-duty@trendmarket"
    assert watcher["threshold"] == "900ms"
    assert watcher["window"] == "1m"
    assert watcher["action"] == "page_dec_oncall"
    assert watcher["kpi"] == "dec.latency.p95"
    assert watcher["description"] == "DEC override"
    assert watcher["rollback"] == "yes"
