#!/usr/bin/env bash
set -euo pipefail
(cd docker && docker compose up -d)
echo
echo "[OK] Stack up. UIs:" \
&& echo "- Prometheus: http://localhost:9090" \
&& echo "- Grafana: http://localhost:3000 (admin/admin)" \
&& echo "- Jaeger UI: http://localhost:16686"
