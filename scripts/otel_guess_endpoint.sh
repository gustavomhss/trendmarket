#!/usr/bin/env bash
set -euo pipefail
export COMPOSE_FILE=docker-compose.observability.yml


# tenta compor a porta HTTP consultando o Compose/PS
get_port_via_compose() {
local p=$1
docker compose port otel-collector "$p" 2>/dev/null | awk -F: 'NF{print $NF}' | tail -n1
}


get_port_via_ps() {
local map
map=$(docker ps --filter name=otel-collector --format '{{.Ports}}' | head -n1)
# extrai candidatos tipo "0.0.0.0:4320->4320/tcp" e devolve o host port preferindo container :4318
echo "$map" | tr ',' '\n' | sed 's/ //g' | awk -F'->' '/->/ {print $1, $2}' |
awk -F: '{
host=$2; r=$NF; if (r ~ /4318\/tcp/) {print host; exit} else if (r ~ /43[0-9]{2}\/tcp/ && r !~ /4317\/tcp/) {print host; exit}
}'
}


# 1) tenta via compose em 4318 e alguns vizinhos comuns
HTTP_PORT=""
for p in 4318 4320 4321 4322; do
hp=$(get_port_via_compose "$p" || true)
if [[ -n "$hp" ]]; then HTTP_PORT="$hp"; break; fi
done


# 2) fallback: parse docker ps
if [[ -z "${HTTP_PORT}" ]]; then
HTTP_PORT=$(get_port_via_ps || true)
fi


# 3) fallback final
HTTP_PORT=${HTTP_PORT:-4318}


mkdir -p .
# injeta/atualiza OTEL_EXPORTER_OTLP_ENDPOINT na .env
if [[ -f ./.env ]]; then
# remove linhas antigas
grep -v '^OTEL_EXPORTER_OTLP_ENDPOINT=' ./.env > ./.env.tmp || true
mv ./.env.tmp ./.env
fi
printf 'OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:%s\n' "$HTTP_PORT" >> ./.env


echo "[guess] endpoint OTLP = http://localhost:${HTTP_PORT} (gravado em .env)"
