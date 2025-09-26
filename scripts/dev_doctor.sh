#!/usr/bin/env bash
set -euo pipefail


need() { command -v "$1" >/dev/null 2>&1 || { echo "[doctor] ❌ falta '$1' no PATH"; exit 2; }; }
need docker


# Tenta falar com o daemon; se falhar, tenta Colima (caso instalado)
if ! docker info >/dev/null 2>&1; then
if command -v colima >/dev/null 2>&1; then
echo "[doctor] docker não respondeu; tentando 'colima start'…"
colima start || true
fi
fi


# Recheca
if ! docker info >/dev/null 2>&1; then
echo "[doctor] ❌ Docker daemon indisponível. Abra o Docker Desktop ou inicie 'colima start'."; exit 2
fi


echo "[doctor] ✅ Docker daemon OK"


# Deteção básica de colisão de portas locais
ports=(4317 4318 9464 16686 14250 9090)
for p in "${ports[@]}"; do
if lsof -i ":$p" -sTCP:LISTEN -Pn 2>/dev/null | grep -q ":$p"; then
echo "[doctor] ⚠️ porta $p já está em uso localmente — pode haver conflito"
fi
done
