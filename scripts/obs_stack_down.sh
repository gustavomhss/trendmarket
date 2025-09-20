#!/usr/bin/env bash
set -euo pipefail
(cd docker && docker compose down -v)
echo "[OK] Stack down (removed)."
