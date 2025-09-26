#!/usr/bin/env bash
set -euo pipefail
chmod +x scripts/*.sh 2>/dev/null || true


# 1) corrige paths de config (arquivo vs diretório)
[ -x scripts/ops_fix_paths.sh ] && ./scripts/ops_fix_paths.sh || true


# 2) hard‑reset (gera compose limpo + .env com portas)
[ -x scripts/compose_hard_reset.sh ] && ./scripts/compose_hard_reset.sh


# 3) sobe stack (idempotente; mostra portas)
./scripts/dev_stack_up.sh


# 4) descobre endpoint real e grava em .env (se mudou)
./scripts/otel_guess_endpoint.sh


# 5) espera endpoint responder
./scripts/otel_preflight_endpoint.sh


# 6) smoke final (tolerância opcional: export TELEMETRY_SHUTDOWN_TOLERANT=1)
./scripts/telemetry_smoke.sh
