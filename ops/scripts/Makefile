ROOT_DIR := $(abspath $(dir $(lastword $(MAKEFILE_LIST)))/..)
PYTHON := python3
REPORTS_DIR := $(ROOT_DIR)/reports
EVIDENCE_DIR := $(ROOT_DIR)/evidence
WATCHERS_CONFIG := $(ROOT_DIR)/watchers/core.yaml
HOOKS_CONFIG := $(ROOT_DIR)/hooks/a110.yml
WATCHERS_REPORT := $(REPORTS_DIR)/watchers_dry.json
HOOKS_REPORT := $(REPORTS_DIR)/hooks_dry.json
REPO_AUDIT_LOG := $(REPORTS_DIR)/repo_audit.json

.PHONY: watchers.dry hooks.dry evidence.publish

watchers.dry:
	@mkdir -p $(REPORTS_DIR)
	@$(PYTHON) $(ROOT_DIR)/scripts/watchers_dry_run.py --config $(WATCHERS_CONFIG) --output $(WATCHERS_REPORT)
	@$(PYTHON) $(ROOT_DIR)/scripts/repo_audit.py --log $(REPO_AUDIT_LOG) --command "make watchers.dry" --status success --artifact $(WATCHERS_REPORT)

hooks.dry:
	@mkdir -p $(REPORTS_DIR)
	@$(PYTHON) $(ROOT_DIR)/scripts/hooks_dry_run.py --config $(HOOKS_CONFIG) --output $(HOOKS_REPORT)
	@$(PYTHON) $(ROOT_DIR)/scripts/repo_audit.py --log $(REPO_AUDIT_LOG) --command "make hooks.dry" --status success --artifact $(HOOKS_REPORT)

# Aggregate watchers/hooks evidence while guaranteeing artefacts exist
# before publishing, em linha com o gate A110.
evidence.publish: watchers.dry hooks.dry
	@mkdir -p $(EVIDENCE_DIR)
	@$(PYTHON) $(ROOT_DIR)/scripts/evidence_publish.py --watchers $(WATCHERS_REPORT) --hooks $(HOOKS_REPORT) --output $(EVIDENCE_DIR)/evidence.json
	@$(PYTHON) $(ROOT_DIR)/scripts/repo_audit.py --log $(REPO_AUDIT_LOG) --command "make evidence.publish" --status success --artifact $(EVIDENCE_DIR)/evidence.json
