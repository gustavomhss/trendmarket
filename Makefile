.PHONY: watchers.dry hooks.dry gate.a110

watchers.dry:
	./scripts/watchers_dry.py

hooks.dry:
	./scripts/hooks_dry.py

gate.a110: watchers.dry hooks.dry
	./scripts/a110_run_invariants.sh
