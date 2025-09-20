#!/usr/bin/env bash
set -euo pipefail


# 0) exige collector up — evita loop bobo
if ! docker ps --format '{{.Names}}' | grep -qx 'otel-collector'; then
echo "[preflight] ❌ collector não está rodando — rode scripts/dev_stack_up.sh"; exit 2
fi


# 1) endpoint
[ -f ./.env ] && set -a && . ./.env && set +a
EP="${OTEL_EXPORTER_OTLP_ENDPOINT:-http://localhost:4318}"
HOST=$(printf %s "$EP" | sed -E 's|^https?://([^:/]+).*|\1|')
PORT=$(printf %s "$EP" | sed -E 's|^https?://[^:/]+:([0-9]+).*|\1|')
PORT=${PORT:-4318}


ok(){ [[ "$1" =~ ^[0-9]{3}$ ]] && [[ "$1" != "000" ]]; }


# 2) espera TCP abrir (127.0.0.1 e ::1, se localhost)
TARGET_HOST="$HOST"
if [ "$HOST" = "localhost" ]; then TARGET_HOST=127.0.0.1; fi


printf "[preflight] aguardando TCP %s:%s…\n" "$TARGET_HOST" "$PORT"
for i in {1..45}; do
if (echo > /dev/tcp/$TARGET_HOST/$PORT) >/dev/null 2>&1; then
echo "[preflight] ✅ socket aberto"; break
fi
sleep 1
if [ $i -eq 45 ]; then echo "[preflight] ❌ porta $PORT não abriu em $TARGET_HOST"; exit 2; fi
done


# 3) HTTP: usar POST vazio p/ obter 400/415 rápido
TR=$(curl -sS -m 2 -o /dev/null -w '%{http_code}' -X POST "$EP/v1/traces" --data-binary '') || TR=000
ME=$(curl -sS -m 2 -o /dev/null -w '%{http_code}' -X POST "$EP/v1/metrics" --data-binary '') || ME=000


if ok "$TR" || ok "$ME"; then
echo "[preflight] ✅ endpoint HTTP acessível (traces=$TR metrics=$ME)"; exit 0
fi


# 4) diagnóstico rápido
echo "[preflight] ❌ HTTP ainda indisponível em $EP (traces=$TR metrics=$ME). Últimos logs:"
docker logs otel-collector --tail=120 2>/dev/null || true
exit 2
