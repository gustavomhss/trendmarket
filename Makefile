SHELL := /bin/sh

PROMETHEUS ?= prometheus
PROMTOOL   ?= promtool
CURL       ?= curl
JQ         ?= jq
PY         ?= python3
YAMLLINT   ?= yamllint
SHELLCHECK ?= shellcheck
RUFF       ?= ruff
RUNNER     ?= scripts/obs_t3_prom_scrape_run.sh
QUALITY    ?= scripts/obs3_quality_checks.py
HASHER     ?= scripts/obs3_hash_manifest.py
VERIFY     ?= scripts/obs3_verify_manifest.py
OUTDIR     ?= out/obs_gatecheck
EVIDIR     ?= $(OUTDIR)/evidence
LOGDIR     ?= $(OUTDIR)/logs
DEV_CFG    ?= ops/prometheus/prometheus.dev.yml
PROD_CFG   ?= ops/prometheus/prometheus.prod.yml
RULES_YML  ?= ops/prometheus/rules/core.rules.yml
TESTS_YML  ?= ops/prometheus/tests/core.rules.test.yml
SCHEMA     ?= ops/schemas/manifest.schema.json
EXAMPLE    ?= ops/schemas/examples/prom_scrape.example.json
LOCAL_VERIFIER ?= scripts/obs3_all_checks.sh

.DEFAULT_GOAL := help

CONFIG_DIR := ops/prometheus
WORKFLOW   := .github/workflows/obs3-prometheus-ci.yml
SCRIPTS_DIR := scripts
PROM_CONFIGS = $(strip $(foreach cfg,$(DEV_CFG) $(PROD_CFG),$(if $(wildcard $(cfg)),$(cfg),)))
RULES_FILE  = $(strip $(if $(wildcard $(RULES_YML)),$(RULES_YML),))
EVIDENCE_FILE ?= $(EVIDIR)/prom_scrape.json

.PHONY: help lint run evidence pr-check stop clean watchers.dry hooks.dry gate.a110 obs.test

define ensure_binary
	command -v $(1) >/dev/null 2>&1 || { echo "Error: missing required tool '$(1)'. $(2)"; exit 1; }
endef

help:
	@printf "Available targets:
"
	@printf "  %-12s %s
" "help" "Show this help message."
	@printf "  %-12s %s
" "lint" "Run local linting for Prometheus configs, YAML, shell, and Python scripts."
	@printf "  %-12s %s
" "run" "Execute the Prometheus scrape runner with the DEV configuration."
	@printf "  %-12s %s
" "evidence" "Generate evidence by running the runner and chained quality/hash/verify steps."
	@printf "  %-12s %s
" "pr-check" "Run the consolidated OBS-3 verifier (all mandatory checks)."
	@printf "  %-12s %s
" "stop" "Stop a locally running Prometheus instance if the PID file exists."
	@printf "  %-12s %s
" "clean" "Reset the OBS-3 output directory and recreate base folders."
	@printf "  %-12s %s
" "watchers.dry" "Execute watcher dry-run checks."
	@printf "  %-12s %s
" "hooks.dry" "Execute hook dry-run checks."
	@printf "  %-12s %s
" "gate.a110" "Run gate A110 invariants."
	@printf "  %-12s %s
" "obs.test" "Run observability feature tests."

lint:
	@echo "==> Lint: verifying required tools"
	@$(call ensure_binary,$(PROMTOOL),Install promtool from https://prometheus.io/docs/prometheus/latest/installation/)
	@$(call ensure_binary,$(YAMLLINT),Install yamllint via 'pip install yamllint' or your package manager.)
	@if ls $(SCRIPTS_DIR)/*.sh >/dev/null 2>&1; then \
		$(call ensure_binary,$(SHELLCHECK),Install shellcheck from https://github.com/koalaman/shellcheck#installing); \
	fi
	@if find $(SCRIPTS_DIR) -name '*.py' -print -quit >/dev/null 2>&1; then \
		$(call ensure_binary,$(RUFF),Install ruff via 'pip install ruff' or your package manager.); \
	fi
	@echo "==> Lint: promtool config checks"
	@if [ -n "$(PROM_CONFIGS)" ]; then \
		echo "Running: $(PROMTOOL) check config $(PROM_CONFIGS)"; \
		$(PROMTOOL) check config $(PROM_CONFIGS); \
	else \
		echo "No Prometheus config files found; skipping"; \
	fi
	@echo "==> Lint: promtool rules checks"
	@if [ -n "$(RULES_FILE)" ]; then \
		echo "Running: $(PROMTOOL) check rules $(RULES_FILE)"; \
		$(PROMTOOL) check rules $(RULES_FILE); \
	else \
		echo "No Prometheus rule file found; skipping"; \
	fi
	@echo "==> Lint: yamllint"
	@if [ -d "$(CONFIG_DIR)" ]; then \
		$(YAMLLINT) $(CONFIG_DIR); \
	else \
		echo "Directory $(CONFIG_DIR) not found; skipping"; \
	fi
	@if [ -f "$(WORKFLOW)" ]; then \
		$(YAMLLINT) $(WORKFLOW); \
	else \
		echo "Workflow $(WORKFLOW) not found; skipping"; \
	fi
	@if ls $(SCRIPTS_DIR)/*.sh >/dev/null 2>&1; then \
		echo "==> Lint: shellcheck"; \
		$(SHELLCHECK) -S warning $(SCRIPTS_DIR)/*.sh; \
	else \
		echo "==> Lint: no shell scripts to lint"; \
	fi
	@if find $(SCRIPTS_DIR) -name '*.py' -print -quit >/dev/null 2>&1; then \
		echo "==> Lint: ruff"; \
		$(RUFF) check $(SCRIPTS_DIR); \
	else \
		echo "==> Lint: no Python scripts to lint"; \
	fi

run:
	@echo "==> Running Prometheus scrape runner (DEV config)"
	@[ -f "$(RUNNER)" ] || { echo "Error: runner '$(RUNNER)' not found."; exit 1; }
	@$(RUNNER) --config "$(DEV_CFG)"

evidence:
	@echo "==> Running runner for evidence (DEV config)"
	@[ -f "$(RUNNER)" ] || { echo "Error: runner '$(RUNNER)' not found."; exit 1; }
	@$(RUNNER) --config "$(DEV_CFG)"
	@if [ -f "$(QUALITY)" ]; then \
		echo "==> Running quality checks"; \
		$(PY) "$(QUALITY)"; \
	else \
		echo "==> Quality script $(QUALITY) not found; skipping"; \
	fi
	@if [ -f "$(HASHER)" ]; then \
		echo "==> Running hash manifest"; \
		$(PY) "$(HASHER)"; \
	else \
		echo "==> Hash manifest script $(HASHER) not found; skipping"; \
	fi
	@if [ -f "$(VERIFY)" ]; then \
		echo "==> Running manifest verifier"; \
		$(PY) "$(VERIFY)"; \
	else \
		echo "==> Manifest verifier $(VERIFY) not found; skipping"; \
	fi

pr-check:
	@echo "==> Running OBS-3 consolidated verifier"
	@[ -f "$(LOCAL_VERIFIER)" ] || { echo "Error: verifier '$(LOCAL_VERIFIER)' not found."; exit 1; }
	@$(LOCAL_VERIFIER)

stop:
	@echo "==> Stopping Prometheus process (if running)"
	@if [ -f "$(LOGDIR)/prom.pid" ]; then \
		pid=$$(cat "$(LOGDIR)/prom.pid"); \
		if kill "$$pid" 2>/dev/null; then \
			echo "Stopped process $$pid"; \
		else \
			echo "No process with PID $$pid (already stopped?)"; \
		fi; \
	else \
		echo "No PID file at $(LOGDIR)/prom.pid"; \
	fi

clean:
	@echo "==> Cleaning $(OUTDIR)"
	@[ "${OUTDIR#out/}" != "$(OUTDIR)" ] || { echo "Error: refusing to clean non-out path: $(OUTDIR)"; exit 1; }
	@rm -rf -- "$(OUTDIR)"
	@mkdir -p "$(EVIDIR)" "$(LOGDIR)"

watchers.dry:
	./scripts/watchers_dry.py

hooks.dry:
	./scripts/hooks_dry.py

gate.a110: watchers.dry hooks.dry
	./scripts/a110_run_invariants.sh

# Observability feature tests sem alterar os defaults.
obs.test:
	cargo test --features obs --no-run
	cargo test --features obs -q
