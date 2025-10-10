#!/usr/bin/env sh

# POSIX shell runner for OBS-3 Thread 4 evidence collection
# shellcheck disable=SC3040,SC3041
set -eu
( set -o pipefail ) 2>/dev/null && set -o pipefail || true
set -E 2>/dev/null || true

usage() {
  cat <<'USAGE'
Usage: obs_t3_prom_scrape_run.sh [options]

Options:
  -c, --config <path>          Prometheus config file (default: ops/prometheus/prometheus.dev.yml)
  -o, --out <dir>              Output directory (default: out/obs_gatecheck)
      --addr <host:port>       Prometheus listen address (default: :9090)
      --retention <dur>        Retention duration (default: 7d)
      --prometheus-bin <bin>   Prometheus binary (default: prometheus)
      --promtool-bin <bin>     promtool binary (default: promtool)
      --curl-bin <bin>         curl binary (default: curl)
      --jq-bin <bin>           jq binary (default: jq)
      --skip-lint              Skip promtool lint checks
      --skip-start             Do not start Prometheus (assume running)
      --no-stop                Do not stop Prometheus on exit
      --ready-attempts <n>     Readiness attempts (default: 30)
      --ready-sleep <s>        Seconds between readiness attempts (default: 1)
      --adhoc-only             Only collect ad-hoc quantiles (skip ce:* queries)
  -h, --help                   Show this help
USAGE
}

log() {
  ts=$(date -u '+%Y-%m-%dT%H:%M:%SZ' 2>/dev/null || date '+%Y-%m-%dT%H:%M:%SZ')
  printf '%s %s\n' "$ts" "$*"
}

err() {
  log "ERROR: $*" >&2
}

info() {
  log "INFO: $*"
}

warn() {
  log "WARN: $*"
}

CONFIG="ops/prometheus/prometheus.dev.yml"
OUT_DIR="out/obs_gatecheck"
ADDR=":9090"
RETENTION="7d"
PROM_BIN="prometheus"
PROMTOOL_BIN="promtool"
CURL_BIN="curl"
JQ_BIN="jq"
SKIP_LINT=0
SKIP_START=0
NO_STOP=0
READY_ATTEMPTS=30
READY_SLEEP=1
ADHOC_ONLY=0

parse_args() {
  while [ "$#" -gt 0 ]; do
    case "$1" in
      -c|--config)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        CONFIG="$2"; shift 2 ;;
      -o|--out)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        OUT_DIR="$2"; shift 2 ;;
      --addr)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        ADDR="$2"; shift 2 ;;
      --retention)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        RETENTION="$2"; shift 2 ;;
      --prometheus-bin)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        PROM_BIN="$2"; shift 2 ;;
      --promtool-bin)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        PROMTOOL_BIN="$2"; shift 2 ;;
      --curl-bin)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        CURL_BIN="$2"; shift 2 ;;
      --jq-bin)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        JQ_BIN="$2"; shift 2 ;;
      --skip-lint)
        SKIP_LINT=1; shift ;;
      --skip-start)
        SKIP_START=1; shift ;;
      --no-stop)
        NO_STOP=1; shift ;;
      --ready-attempts)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        READY_ATTEMPTS="$2"; shift 2 ;;
      --ready-sleep)
        [ $# -ge 2 ] || { err "Missing value for $1"; usage; exit 1; }
        READY_SLEEP="$2"; shift 2 ;;
      --adhoc-only)
        ADHOC_ONLY=1; shift ;;
      -h|--help)
        usage; exit 0 ;;
      --)
        shift; break ;;
      -*)
        err "Unknown option: $1"; usage; exit 1 ;;
      *)
        err "Unexpected argument: $1"; usage; exit 1 ;;
    esac
  done
}

parse_args "$@"

if [ ! -f "$CONFIG" ]; then
  err "Config file not found: $CONFIG"
  exit 5
fi

if [ "$READY_ATTEMPTS" -le 0 ] 2>/dev/null; then
  err "--ready-attempts must be positive"
  exit 1
fi

if [ "$READY_SLEEP" -le 0 ] 2>/dev/null; then
  err "--ready-sleep must be positive"
  exit 1
fi

mkdir -p "$OUT_DIR/logs" "$OUT_DIR/evidence"
LOG_DIR="$OUT_DIR/logs"
EVIDENCE_DIR="$OUT_DIR/evidence"
PROM_LOG="$LOG_DIR/prometheus.txt"
PROM_CHECK_LOG="$LOG_DIR/prom_check.txt"
PROM_PID_FILE="$LOG_DIR/prom.pid"
DATA_DIR="$OUT_DIR/prom-data"

addr_host="${ADDR%:*}"
addr_port="${ADDR##*:}"
if [ "$ADDR" = "$addr_port" ]; then
  err "--addr must be in host:port format"
  exit 1
fi

if [ -z "$addr_host" ] || [ "$addr_host" = "*" ]; then
  warn "Empty host in --addr; forcing 127.0.0.1 for safety"
  addr_host="127.0.0.1"
fi
case "$addr_host" in
  127.*|localhost)
    : ;;
  ::1)
    : ;;
  *)
    err "Refusing to bind to non-loopback host: $addr_host"
    exit 1 ;;
esac

LISTEN_ADDR="$addr_host:$addr_port"
BASE_URL="http://$addr_host:$addr_port"

check_bin() {
  bin_path="$1"
  name="$2"
  if ! command -v "$bin_path" >/dev/null 2>&1; then
    err "$name binary not found: $bin_path"
    exit 7
  fi
  version_output=""
  if "$bin_path" --version >/dev/null 2>&1; then
    version_output=$("$bin_path" --version 2>&1 | head -n 1)
  elif "$bin_path" -V >/dev/null 2>&1; then
    version_output=$("$bin_path" -V 2>&1 | head -n 1)
  elif "$bin_path" -v >/dev/null 2>&1; then
    version_output=$("$bin_path" -v 2>&1 | head -n 1)
  else
    version_output="version info unavailable"
  fi
  info "$name: $version_output"
}

check_bin "$PROM_BIN" "prometheus"
check_bin "$PROMTOOL_BIN" "promtool"
check_bin "$CURL_BIN" "curl"
check_bin "$JQ_BIN" "jq"
check_bin grep "grep"
check_bin awk "awk"
check_bin sed "sed"
check_bin date "date"

PROM_STARTED=0
PROM_PID=0

cleanup() {
  rc=$1
  if [ "$PROM_STARTED" -eq 1 ] && [ "$NO_STOP" -eq 0 ]; then
    if kill -0 "$PROM_PID" >/dev/null 2>&1; then
      info "Stopping Prometheus (pid $PROM_PID)"
      kill "$PROM_PID" >/dev/null 2>&1 || true
      wait "$PROM_PID" 2>/dev/null || true
    fi
  fi
  exit "$rc"
}

trap 'rc=$?; cleanup "$rc"' EXIT HUP INT TERM

run_lint() {
  if [ "$SKIP_LINT" -eq 1 ]; then
    info "Skipping promtool lint checks"
    printf 'promtool lint skipped (--skip-lint)\n' >"$PROM_CHECK_LOG"
    return
  fi
  info "Running promtool lint checks (logging to $PROM_CHECK_LOG)"
  {
    if ! "$PROMTOOL_BIN" check config "$CONFIG"; then
      exit 1
    fi
    RULE_FILE="ops/prometheus/rules/core.rules.yml"
    if [ -f "$RULE_FILE" ]; then
      if ! "$PROMTOOL_BIN" check rules "$RULE_FILE"; then
        exit 2
      fi
    else
      printf 'Rules file %s not found; skipping rules check\n' "$RULE_FILE"
    fi
  } >"$PROM_CHECK_LOG" 2>&1
  lint_rc=$?
  if [ "$lint_rc" -ne 0 ]; then
    err "promtool checks failed (see $PROM_CHECK_LOG)"
    exit 2
  fi
}

start_prometheus() {
  if [ "$SKIP_START" -eq 1 ]; then
    info "Skipping Prometheus start; expecting instance at $BASE_URL"
    return
  fi
  mkdir -p "$DATA_DIR"
  info "Starting Prometheus using $CONFIG at $LISTEN_ADDR"
  "$PROM_BIN" \
    --config.file="$CONFIG" \
    --storage.tsdb.path="$DATA_DIR" \
    --storage.tsdb.retention.time="$RETENTION" \
    --web.listen-address="$LISTEN_ADDR" \
    >"$PROM_LOG" 2>&1 &
  PROM_PID=$!
  PROM_STARTED=1
  printf '%s\n' "$PROM_PID" >"$PROM_PID_FILE"
  info "Prometheus started with PID $PROM_PID"
}

wait_readiness() {
  info "Waiting for Prometheus readiness at $BASE_URL/-/ready"
  attempt=1
  while [ "$attempt" -le "$READY_ATTEMPTS" ]; do
    HTTP_CODE=$($CURL_BIN -sS -o /dev/null -w '%{http_code}' "$BASE_URL/-/ready" || true)
    if [ "$HTTP_CODE" = "200" ]; then
      info "Prometheus is ready (attempt $attempt)"
      return
    fi
    sleep "$READY_SLEEP"
    attempt=$((attempt + 1))
  done
  err "Prometheus readiness failed after $READY_ATTEMPTS attempts"
  if [ -f "$PROM_LOG" ]; then
    warn "Last 50 lines of Prometheus log:"
    tail -n 50 "$PROM_LOG" >&2
  fi
  exit 3
}

fetch_endpoint() {
  file_name="$1"
  endpoint="$2"
  shift 2
  tmp_file="$EVIDENCE_DIR/$file_name.tmp"
  target_file="$EVIDENCE_DIR/$file_name"
  info "Fetching $endpoint -> $target_file"
  if ! $CURL_BIN -sS --fail "$BASE_URL$endpoint" "$@" -o "$tmp_file"; then
    err "Failed to fetch $endpoint"
    rm -f "$tmp_file"
    exit 5
  fi
  mv "$tmp_file" "$target_file"
}

fetch_query() {
  file_name="$1"
  endpoint="$2"
  shift 2
  tmp_file="$EVIDENCE_DIR/$file_name.tmp"
  target_file="$EVIDENCE_DIR/$file_name"
  info "Querying $endpoint -> $target_file"
  if ! $CURL_BIN -sS --fail --get "$BASE_URL$endpoint" "$@" -o "$tmp_file"; then
    err "Failed to query $endpoint"
    rm -f "$tmp_file"
    exit 5
  fi
  mv "$tmp_file" "$target_file"
}

fail_fast() {
  info "Running minimal dataset checks"
  up_status=$($JQ_BIN -r '.status' "$EVIDENCE_DIR/prom_up.json")
  if [ "$up_status" != "success" ]; then
    err "prom_up query did not return success"
    exit 6
  fi
  up_ready=$($JQ_BIN '.data.result | map(select(.value[1] == "1")) | length' "$EVIDENCE_DIR/prom_up.json")
  if [ "$up_ready" -lt 1 ]; then
    err "No targets reporting up=1"
    exit 6
  fi

  recorded_ok=0
  adhoc_ok=0

  if [ "$ADHOC_ONLY" -eq 0 ] && [ -f "$EVIDENCE_DIR/prom_p75_rec.json" ]; then
    rec_status=$($JQ_BIN -r '.status' "$EVIDENCE_DIR/prom_p75_rec.json")
    rec_count=$($JQ_BIN '.data.result | length' "$EVIDENCE_DIR/prom_p75_rec.json")
    if [ "$rec_status" = "success" ] && [ "$rec_count" -gt 0 ]; then
      recorded_ok=1
    fi
  fi

  if [ -f "$EVIDENCE_DIR/prom_p75_adhoc.json" ]; then
    adhoc_status=$($JQ_BIN -r '.status' "$EVIDENCE_DIR/prom_p75_adhoc.json")
    adhoc_count=$($JQ_BIN '.data.result | length' "$EVIDENCE_DIR/prom_p75_adhoc.json")
    if [ "$adhoc_status" = "success" ] && [ "$adhoc_count" -gt 0 ]; then
      adhoc_ok=1
    fi
  fi

  if [ "$recorded_ok" -eq 0 ] && [ "$adhoc_ok" -eq 0 ]; then
    err "No usable latency datasets (recorded or ad-hoc)"
    exit 6
  fi

  series_bucket_count=$($JQ_BIN '[.data[] | select(.__name__ == "amm_op_latency_seconds_bucket")] | length' "$EVIDENCE_DIR/prom_series.json")
  if [ "$series_bucket_count" -lt 1 ]; then
    err "prom_series does not include amm_op_latency_seconds_bucket"
    exit 6
  fi
}

create_skipped_recorded() {
  file_name="$1"
  info "Skipping recorded query $file_name due to --adhoc-only"
  cat <<'JSON' >"$EVIDENCE_DIR/$file_name"
{"status":"skipped","errorType":"adhoc_only","error":"recorded quantile queries disabled"}
JSON
}

run_optional() {
  script="$1"
  if [ -f "$script" ]; then
    if ! command -v python3 >/dev/null 2>&1; then
      err "python3 is required to run $script"
      exit 7
    fi
    info "Running optional integration: $script"
    if ! python3 "$script"; then
      rc=$?
      err "$script failed with exit code $rc"
      exit "$rc"
    fi
  else
    info "Optional script not found, skipping: $script"
  fi
}

run_lint
start_prometheus
wait_readiness

fetch_endpoint "prom_targets.json" "/api/v1/targets"
fetch_endpoint "prom_rules.json" "/api/v1/rules"
fetch_query "prom_up.json" "/api/v1/query" --data-urlencode "query=up"

if [ "$ADHOC_ONLY" -eq 0 ]; then
  fetch_query "prom_p75_rec.json" "/api/v1/query" --data-urlencode "query=ce:amm_op_latency_seconds:p75"
  fetch_query "prom_p95_rec.json" "/api/v1/query" --data-urlencode "query=ce:amm_op_latency_seconds:p95"
else
  create_skipped_recorded "prom_p75_rec.json"
  create_skipped_recorded "prom_p95_rec.json"
fi

fetch_query "prom_p75_adhoc.json" "/api/v1/query" --data-urlencode "query=histogram_quantile(0.75, sum by (le,op,service) (rate(amm_op_latency_seconds_bucket[5m])))"
fetch_query "prom_p95_adhoc.json" "/api/v1/query" --data-urlencode "query=histogram_quantile(0.95, sum by (le,op,service) (rate(amm_op_latency_seconds_bucket[5m])))"

now=$(date -u +%s 2>/dev/null || date +%s)
start=$((now - 600))
fetch_query "prom_series.json" "/api/v1/series" \
  --data-urlencode "match[]=amm_op_latency_seconds_bucket" \
  --data-urlencode "match[]=amm_op_latency_seconds_sum" \
  --data-urlencode "match[]=amm_op_latency_seconds_count" \
  --data-urlencode "match[]=hook_executions_total" \
  --data-urlencode "start=$start" \
  --data-urlencode "end=$now"

fail_fast

run_optional "scripts/obs3_quality_checks.py"
run_optional "scripts/obs3_hash_manifest.py"
run_optional "scripts/obs3_verify_manifest.py"

info "Evidence collected at $EVIDENCE_DIR"
exit 0
