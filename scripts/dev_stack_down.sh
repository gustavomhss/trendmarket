#!/usr/bin/env bash
set -euo pipefail
export COMPOSE_FILE=docker-compose.observability.yml
docker compose down -v
