#!/usr/bin/env bash
set -euo pipefail

COLLECTOR_BIN=".tools/otelcol-contrib"
CONFIG_FILE="ops/otel/collector-dev.trace.yaml"
STUB_RUNNER="scripts/otelcol_trace_stub.py"
LOG_DIR="out/obs_gatecheck/logs"
STDOUT_LOG="${LOG_DIR}/otelcol_trace.out"
STDERR_LOG="${LOG_DIR}/otelcol_trace.err"
PID_FILE="${LOG_DIR}/otelcol_trace.pid"
VALIDATE_LOG="${LOG_DIR}/obs4_thread02_validate.txt"
LOG_DUMP="${LOG_DIR}/obs4_thread02_logs_dump.txt"

ensure_dirs() {
  mkdir -p "${LOG_DIR}"
}

has_real_collector() {
  python3 - "$COLLECTOR_BIN" <<'PY'
import os
import sys

path = sys.argv[1]
if not os.path.exists(path):
    sys.exit(1)
with open(path, 'rb') as fh:
    magic = fh.read(4)
if magic == b"\x7fELF":
    sys.exit(0)
sys.exit(1)
PY
}

collector_pid() {
  if [[ -f "${PID_FILE}" ]]; then
    local pid
    pid="$(cat "${PID_FILE}" 2>/dev/null || true)"
    if [[ -n "${pid}" && -d "/proc/${pid}" ]]; then
      echo "${pid}"
      return 0
    fi
  fi
  return 1
}

stop_collector() {
  if pid=$(collector_pid); then
    kill "${pid}" 2>/dev/null || true
    for _ in $(seq 1 30); do
      if [[ ! -d "/proc/${pid}" ]]; then
        break
      fi
      sleep 1
    done
    if [[ -d "/proc/${pid}" ]]; then
      kill -9 "${pid}" 2>/dev/null || true
    fi
    rm -f "${PID_FILE}"
    return 0
  fi
  rm -f "${PID_FILE}"
  return 0
}

cmd_validate() {
  ensure_dirs
  if has_real_collector; then
    if "${COLLECTOR_BIN}" validate --config "${CONFIG_FILE}" | tee "${VALIDATE_LOG}"; then
      exit 0
    else
      exit 1
    fi
  else
    if python3 "${STUB_RUNNER}" --config "${CONFIG_FILE}" --stub-validate | tee "${VALIDATE_LOG}"; then
      exit 0
    else
      exit 1
    fi
  fi
}

cmd_start() {
  ensure_dirs
  stop_collector
  : > "${STDOUT_LOG}"
  : > "${STDERR_LOG}"

  local pid
  if has_real_collector; then
    "${COLLECTOR_BIN}" --config "${CONFIG_FILE}" \
      >>"${STDOUT_LOG}" 2>>"${STDERR_LOG}" &
    pid=$!
  else
    python3 "${STUB_RUNNER}" --config "${CONFIG_FILE}" \
      >>"${STDOUT_LOG}" 2>>"${STDERR_LOG}" &
    pid=$!
  fi
  echo "${pid}" > "${PID_FILE}"

  local addr="${OTELCOL_LISTEN_ADDR:-127.0.0.1}"
  local port="${OTELCOL_LISTEN_PORT:-8888}"
  local metrics_url="http://${addr}:${port}/metrics"
  local success=1

  for _ in $(seq 1 60); do
    if curl -fsS --max-time 2 "${metrics_url}" >/dev/null 2>&1; then
      success=0
      break
    fi
    if [[ ! -d "/proc/${pid}" ]]; then
      break
    fi
    sleep 1
  done

  if [[ ${success} -ne 0 ]]; then
    echo "Collector failed to become healthy at ${metrics_url} within 60s" >&2
    tail -n 200 "${STDERR_LOG}" >&2 || true
    stop_collector || true
    exit 4
  fi
}

cmd_stop() {
  ensure_dirs
  if stop_collector; then
    echo "Collector stopped"
  else
    echo "Collector not running"
  fi
}

cmd_status() {
  ensure_dirs
  local addr="${OTELCOL_LISTEN_ADDR:-127.0.0.1}"
  local port="${OTELCOL_LISTEN_PORT:-8888}"
  local metrics_url="http://${addr}:${port}/metrics"

  if pid=$(collector_pid); then
    if curl -fsS --max-time 2 "${metrics_url}" >/dev/null 2>&1; then
      echo "Collector running (pid ${pid}) - metrics on ${metrics_url}"
    else
      echo "Collector running (pid ${pid}) - metrics endpoint not responding" >&2
      exit 2
    fi
  else
    echo "Collector not running"
    exit 3
  fi
}

case "${1:-status}" in
  validate)
    cmd_validate
    ;;
  start)
    cmd_start
    ;;
  stop)
    cmd_stop
    ;;
  status)
    cmd_status
    ;;
  *)
    echo "Usage: $0 {validate|start|stop|status}" >&2
    exit 64
    ;;
esac
