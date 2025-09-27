import importlib.util
import json
from pathlib import Path

import pytest


def _load_module():
    module_path = Path(__file__).resolve().parents[1] / "watchers_dry.py"
    spec = importlib.util.spec_from_file_location("watchers_dry", module_path)
    if spec is None or spec.loader is None:  # pragma: no cover - defensive
        raise RuntimeError("Unable to load watchers_dry module")

    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


watchers_dry = _load_module()


def _write_aggregated_inventory(base_dir: Path, filename: str = "core.yaml") -> Path:
    watchers_dir = base_dir / "ops" / "watchers"
    watchers_dir.mkdir(parents=True, exist_ok=True)

    payload = {
        "version": 1,
        "domains": {
            domain: sorted(expected)
            for domain, expected in watchers_dry.EXPECTED_DOMAIN_WATCHERS.items()
        },
    }

    path = watchers_dir / filename
    path.write_text(json.dumps(payload, indent=2))
    return watchers_dir


def _write_domain_file(watchers_dir: Path, domain: str) -> None:
    domain_watchers = []
    for name in sorted(watchers_dry.EXPECTED_DOMAIN_WATCHERS[domain]):
        domain_watchers.append(
            {
                "name": name,
                "kpi": "metric",
                "threshold": "threshold",
                "window": "5m",
                "action": "action",
                "owner": "owner@trendmarket",
                "rollback": "yes",
            }
        )

    payload = {"domain": domain, "watchers": domain_watchers}
    (watchers_dir / f"{domain.lower()}.yml").write_text(json.dumps(payload, indent=2))


def test_load_watchers_file_supports_aggregated_payload(tmp_path: Path) -> None:
    payload = {"domains": {"DEC": ["a", "b"]}}
    path = tmp_path / "core.yaml"
    path.write_text(json.dumps(payload))

    result, aggregated = watchers_dry._load_watchers_file(path)

    assert result == {"DEC": {"a", "b"}}
    assert aggregated is True


def test_load_watchers_file_for_domain_payload(tmp_path: Path) -> None:
    payload = {
        "domain": "DEC",
        "watchers": [
            {
                "name": "metrics_decision_hook_gap_watch",
                "kpi": "metric",
                "threshold": "value",
                "window": "5m",
                "action": "act",
                "owner": "owner@trendmarket",
                "rollback": "yes",
            }
        ],
    }
    path = tmp_path / "dec.yml"
    path.write_text(json.dumps(payload))

    result, aggregated = watchers_dry._load_watchers_file(path)

    assert result == {"DEC": {"metrics_decision_hook_gap_watch"}}
    assert aggregated is False


def test_main_accepts_aggregated_inventory(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]) -> None:
    _write_aggregated_inventory(tmp_path)

    monkeypatch.chdir(tmp_path)
    exit_code = watchers_dry.main()

    captured = capsys.readouterr()
    assert exit_code == 0
    assert "[watchers.dry] watcher coverage OK:" in captured.out


def test_main_allows_per_domain_files_with_aggregated_inventory(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    watchers_dir = _write_aggregated_inventory(tmp_path)
    _write_domain_file(watchers_dir, "DEC")

    monkeypatch.chdir(tmp_path)
    exit_code = watchers_dry.main()

    captured = capsys.readouterr()
    assert exit_code == 0
    assert "DEC: 3 watcher(s)" in captured.out


def test_main_handles_aggregated_inventory_loaded_after_domain(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    watchers_dir = _write_aggregated_inventory(tmp_path, filename="zzz_core.yaml")
    _write_domain_file(watchers_dir, "DEC")

    monkeypatch.chdir(tmp_path)
    watchers_files = sorted(path.name for path in (tmp_path / "ops" / "watchers").iterdir())
    assert watchers_files == ["dec.yml", "zzz_core.yaml"]

    exit_code = watchers_dry.main()

    assert exit_code == 0


def test_main_rejects_duplicate_per_domain_files_when_aggregated_present(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    watchers_dir = _write_aggregated_inventory(tmp_path)
    _write_domain_file(watchers_dir, "DEC")
    duplicate_payload = {
        "domain": "DEC",
        "watchers": [
            {
                "name": "metrics_decision_hook_gap_watch",
                "kpi": "metric",
                "threshold": "threshold",
                "window": "5m",
                "action": "action",
                "owner": "owner@trendmarket",
                "rollback": "yes",
            }
        ],
    }
    (watchers_dir / "dec_duplicate.yml").write_text(json.dumps(duplicate_payload, indent=2))

    monkeypatch.chdir(tmp_path)
    exit_code = watchers_dry.main()

    captured = capsys.readouterr()
    assert exit_code == 1
    assert "duplicate watcher definition for domain 'DEC'" in captured.err
