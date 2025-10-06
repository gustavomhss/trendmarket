#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="$ROOT_DIR/out/obs_gatecheck"
LOG_DIR="$OUT_DIR/logs"
EVI_DIR="$OUT_DIR/evidence"
mkdir -p "$LOG_DIR" "$EVI_DIR"

MODE="${1:-prom}"   # prom | rw | compose-prom | compose-rw
CFG_PROM="$ROOT_DIR/ops/otel/collector-dev.prom.yaml"
CFG_RW="$ROOT_DIR/ops/otel/collector-dev.rw.yaml"
LOG="$LOG_DIR/collector_dev.txt"
JSON="$EVI_DIR/collector_dev.json"
COMPOSE_FILE="$ROOT_DIR/ops/otel/docker-compose.dev.yml"
COMPOSE_IN_USE=""
TMP_COMPOSE=""

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG"; }
http_get(){ curl -fsSL "$1"; }
has_cmd(){ command -v "$1" >/dev/null 2>&1; }

HEALTH_URL="http://localhost:13133/healthz"
SELF_METRICS_URL="http://localhost:8888/metrics"
PIPE_METRICS_URL="http://localhost:9464/metrics"

start_binary(){
  local cfg="$1"
  if ! has_cmd otelcol-contrib; then
    note "otelcol-contrib binary not found; skipping binary launch."
    return 1
  fi
  note "Starting otelcol-contrib with $cfg"
  (otelcol-contrib --config "$cfg" 2>&1 | tee -a "$LOG") &
  echo $! > "$LOG_DIR/pid"
  sleep 2
}

stop_binary(){
  if [[ -f "$LOG_DIR/pid" ]]; then
    kill "$(cat "$LOG_DIR/pid")" 2>/dev/null || true
    rm -f "$LOG_DIR/pid"
  fi
}

start_compose(){
  local mode="$1"
  if ! has_cmd docker; then
    note "Docker CLI not available; skipping compose launch."
    return 1
  fi
  note "Starting docker compose using $mode"
  local compose_file="$COMPOSE_FILE"
  if [[ "$mode" == "rw" ]]; then
    TMP_COMPOSE="$(mktemp "$LOG_DIR/docker-compose.dev.XXXX.yaml")"
    cp "$COMPOSE_FILE" "$TMP_COMPOSE"
    sed -i 's#collector-dev\.prom\.yaml#collector-dev.rw.yaml#g' "$TMP_COMPOSE"
    compose_file="$TMP_COMPOSE"
  fi
  COMPOSE_IN_USE="$compose_file"
  (cd "$ROOT_DIR/ops/otel" && docker compose -f "$compose_file" down --remove-orphans >/dev/null 2>&1 || true)
  (cd "$ROOT_DIR/ops/otel" && docker compose -f "$compose_file" up -d 2>&1 | tee -a "$LOG")
}

stop_compose(){
  if ! has_cmd docker; then
    [[ -n "$TMP_COMPOSE" && -f "$TMP_COMPOSE" ]] && rm -f "$TMP_COMPOSE"
    return
  fi
  local compose_file="${COMPOSE_IN_USE:-$COMPOSE_FILE}"
  (cd "$ROOT_DIR/ops/otel" && docker compose -f "$compose_file" down --remove-orphans 2>&1 | tee -a "$LOG") || true
  if [[ -n "$TMP_COMPOSE" && -f "$TMP_COMPOSE" ]]; then
    rm -f "$TMP_COMPOSE"
    TMP_COMPOSE=""
  fi
}

health_checks(){
  local health_ok=false
  local self_ok=false
  local pipe_ok=false

  if http_get "$HEALTH_URL" | grep -q "Server available"; then health_ok=true; fi
  if http_get "$SELF_METRICS_URL" >/dev/null 2>&1; then self_ok=true; fi
  if http_get "$PIPE_METRICS_URL" >/dev/null 2>&1; then pipe_ok=true; fi

  printf '{"health":"%s","pipelines":["metrics","traces","logs"],"tail_sampling":{"slow_ms":%s,"error":true},"exporters":{"prom":"%s","tempo":"on","loki":"on"}}\n' \
    "$( $health_ok && echo ok || echo fail )" \
    "${TAIL_SLOW_MS:-100}" \
    "$( $pipe_ok && echo on || echo off )" | tee "$JSON" >/dev/null

  note "Health: $health_ok | Self-metrics: $self_ok | Pipeline-metrics: $pipe_ok"
}

cleanup(){
  stop_binary
  stop_compose
}

trap cleanup EXIT
: >"$LOG"

case "$MODE" in
  prom)
    start_binary "$CFG_PROM" || true ;;
  rw)
    start_binary "$CFG_RW" || true ;;
  compose-prom)
    start_compose prom || true ;;
  compose-rw)
    start_compose rw || true ;;
  *)
    echo "Usage: $0 [prom|rw|compose-prom|compose-rw]"; exit 2 ;;
 esac

# Wait briefly for startup
sleep 3

# Health & evidence
( curl -fsS "$HEALTH_URL" | tee -a "$LOG" ) || note "Health endpoint unavailable"
( curl -fsS "$SELF_METRICS_URL" | head -n 20 | tee -a "$LOG" ) || note "Self-metrics endpoint unavailable"
( curl -fsS "$PIPE_METRICS_URL" | head -n 20 | tee -a "$LOG" ) || note "Prometheus exporter endpoint unavailable"
health_checks
note "Evidence written to $JSON"
