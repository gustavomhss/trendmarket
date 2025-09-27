import importlib.util
import json
import pathlib

import pytest


ROOT_DIR = pathlib.Path(__file__).resolve().parents[3]
MODULE_PATH = ROOT_DIR / "scripts" / "watchers_dry.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("watchers_dry", MODULE_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError("Unable to load watchers_dry module")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


watchers_dry = _load_module()


def _write_core(payload, directory: pathlib.Path) -> pathlib.Path:
    path = directory / "core.yaml"
    path.write_text(json.dumps(payload), encoding="utf-8")
    return path


def _build_core_payload(drop: tuple[str, str] | None = None) -> dict:
    domains = {
        domain: sorted(watchers)
        for domain, watchers in watchers_dry.EXPECTED_DOMAIN_WATCHERS.items()
    }
    if drop is not None:
        domain, watcher_name = drop
        domains[domain] = [name for name in domains[domain] if name != watcher_name]
    return {"version": 1, "domains": domains}


@pytest.fixture()
def watchers_tmpdir(tmp_path, monkeypatch):
    root = tmp_path / "repo"
    watchers_dir = root / "ops" / "watchers"
    watchers_dir.mkdir(parents=True)
    monkeypatch.chdir(root)
    return watchers_dir


def test_core_yaml_missing_required_watcher_fails(capsys, watchers_tmpdir):
    _write_core(_build_core_payload(drop=("DEC", "model_drift_watch")), watchers_tmpdir)

    exit_code = watchers_dry.main()

    captured = capsys.readouterr()
    assert exit_code == 1
    assert "domain 'DEC' missing required watcher(s): model_drift_watch" in captured.err


def test_core_yaml_restored_watcher_passes(capsys, watchers_tmpdir):
    _write_core(_build_core_payload(), watchers_tmpdir)

    exit_code = watchers_dry.main()

    captured = capsys.readouterr()
    assert exit_code == 0
    assert "[watchers.dry] watcher coverage OK" in captured.out
