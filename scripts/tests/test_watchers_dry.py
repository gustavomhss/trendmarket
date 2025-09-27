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


def _write_aggregated_inventory(base_dir: Path) -> Path:
    watchers_dir = base_dir / "ops" / "watchers"
    watchers_dir.mkdir(parents=True, exist_ok=True)

    payload = {
        "version": 1,
        "domains": {
            domain: sorted(expected)
            for domain, expected in watchers_dry.EXPECTED_DOMAIN_WATCHERS.items()
        },
    }

    path = watchers_dir / "core.yaml"
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

    result = watchers_dry._load_watchers_file(path)

    assert result == {"DEC": {"a", "b"}}


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
