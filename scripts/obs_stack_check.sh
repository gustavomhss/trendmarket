#!/usr/bin/env bash
set -euo pipefail
if curl -fsS http://localhost:9464/metrics | head -n 5; then
echo "\n[OK] /metrics acessível no host."
else
echo "[WARN] /metrics indisponível. Verifique se o bin está rodando com features obs/obs-bin." >&2
exit 1
fi
