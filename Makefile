.PHONY: help lint run evidence stop clean pr-check watchers.dry hooks.dry gate.a110

help:
@echo "Available targets:"
@echo "  make help           # show this message"
@echo "  make lint           # run python/yaml/shell linters"
@echo "  make run            # execute OBS-3 scrape run against dev"
@echo "  make evidence       # alias for make run"
@echo "  make stop           # stop OBS-3 ancillary processes"
@echo "  make clean          # remove generated evidence artifacts"
@echo "  make pr-check       # run canonical OBS-3 validation suite"
@echo "  make watchers.dry   # existing watcher dry-run"
@echo "  make hooks.dry      # existing hook dry-run"
@echo "  make gate.a110      # existing gate sequence"

lint:
python -m pip install -r requirements.txt >/dev/null
ruff check scripts ops/prometheus
yamllint ops/prometheus ops/schemas .github/workflows
shellcheck scripts/obs_t3_prom_scrape_run.sh scripts/obs3_all_checks.sh scripts/gh_setup_repo_policies.sh

run evidence:
./scripts/obs_t3_prom_scrape_run.sh --env dev

stop:
@echo "No long-running OBS-3 processes to stop; ensure external Prometheus is managed separately."

clean:
rm -rf out/obs_gatecheck

pr-check: lint
./scripts/obs3_all_checks.sh

watchers.dry:
./scripts/watchers_dry.py

hooks.dry:
./scripts/hooks_dry.py

gate.a110: watchers.dry hooks.dry
./scripts/a110_run_invariants.sh
