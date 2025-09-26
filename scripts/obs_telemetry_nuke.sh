#!/usr/bin/env bash
# Zera tudo que criamos nesta task (compose, ops/otel, scripts auxiliares, .env vars, deps OTel no Cargo),
# sem bagunçar o resto do repo. Opcional: reset HARD pro origin/main.
set -euo pipefail

echo "==> [1/8] Parando/removendo containers de observabilidade (se existirem)…"
if [ -f docker-compose.observability.yml ]; then
  docker compose -f docker-compose.observability.yml down -v --remove-orphans || true
fi
for c in otel-collector jaeger prometheus; do
  docker rm -f "$c" >/dev/null 2>&1 || true
done
docker network rm credit-engine-core_default >/dev/null 2>&1 || true

echo "==> [2/8] Backup opcional das mudanças (branch + stash)…"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git status --porcelain | grep . >/dev/null 2>&1 && {
    git branch "backup/telemetry_pre_nuke_$(date +%Y%m%d_%H%M%S)" || true
    git stash push -u -m "pre-nuke telemetry stack" || true
  }
fi

echo "==> [3/8] Removendo arquivos/diretórios criados pela task…"
rm -f  docker-compose.observability.yml || true
rm -rf ops/otel || true

# scripts que criamos durante a task
rm -f scripts/otel_*.sh              2>/dev/null || true
rm -f scripts/telemetry_*.sh         2>/dev/null || true
rm -f scripts/compose_*strict*.sh    2>/dev/null || true
rm -f scripts/compose_hard_reset*.sh 2>/dev/null || true
rm -f scripts/dev_stack_up.sh        2>/dev/null || true
rm -f scripts/obs_enable_healthz*.sh 2>/dev/null || true
rm -f scripts/obs_e2e*.sh            2>/dev/null || true
rm -f scripts/obs_stack_doctor*.sh   2>/dev/null || true
rm -f scripts/obs_collect_artifacts*.sh 2>/dev/null || true
rm -f scripts/obs_make_jira_pkg*.sh  2>/dev/null || true
rm -f scripts/obs_require_feature_bin.sh 2>/dev/null || true
rm -f scripts/ops_fix_paths*.sh      2>/dev/null || true

echo "==> [4/8] Limpando bin/telemetry e módulo de exemplo (se criados pela task)…"
rm -f src/bin/telemetry_smoke.rs 2>/dev/null || true
# Se você adicionou src/telemetry/mod.rs só pra task, e NÃO fazia parte do repo:
# Descomente a linha abaixo para remover:
# rm -rf src/telemetry 2>/dev/null || true

echo "==> [5/8] Revertendo Cargo.toml/Cargo.lock para o baseline do repositório (se houver git)…"
if git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  git checkout -- Cargo.toml Cargo.lock 2>/dev/null || true
  # também tenta restaurar arquivos que poderíamos ter criado/alterado
  git checkout -- docker-compose.observability.yml ops/otel src/bin/telemetry_smoke.rs src/telemetry 2>/dev/null || true
fi

echo "==> [6/8] Limpando variáveis desta task no .env (mantendo o resto)…"
if [ -f .env ]; then
  cp .env ".env.bak.$(date +%Y%m%d_%H%M%S)"
  # remove apenas as linhas que nós introduzimos
  grep -v -E '^(HTTP_PORT|GRPC_PORT|JAEGER_UI|JAEGER_GRPC|PROM_WEB|OTEL_EXPORTER_OTLP_ENDPOINT)=' .env > .env.__tmp || true
  mv .env.__tmp .env
fi

echo "==> [7/8] Limpando build local (cargo clean)…"
cargo clean 2>/dev/null || true

echo "==> [8/8] Checagem básica do projeto (opcional)…"
if command -v cargo >/dev/null 2>&1; then
  cargo check -q || true
fi

cat <<'MSG'

✅ Nuke concluído.

Você está de volta ao baseline local. O que pode ter acontecido:
- Se o repo é git: suas modificações foram salvas em stash (e branch backup/*). Cargo.toml/Cargo.lock foram restaurados.
- Todo o stack de observabilidade (compose, collector, prometheus, scripts) foi removido.
- .env foi saneado (linhas desta task removidas).

⚠️ RESET TOTAL (opcional, só se quiser voltar ao estado exato do remote):
   git fetch origin && git reset --hard origin/main
   (Isso descarta TUDO que não estiver no remote.)

MSG
