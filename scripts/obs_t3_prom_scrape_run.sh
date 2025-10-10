#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage: $0 [--env dev|prod] [--config PATH] [--prometheus-url URL]

Run OBS-3 scrape validation against a Prometheus endpoint. The script
validates configuration, executes promtool rule tests, collects telemetry,
and produces evidence bundles + manifests required by the OBS-3 gate.
USAGE
}

log() {
  printf '[%s] %s\n' "$(date --iso-8601=seconds)" "$*"
}

require_cmd() {
  local cmd="$1"
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
}

ENVIRONMENT="dev"
CONFIG=""
PROM_URL=${PROMETHEUS_URL:-"http://localhost:9090"}
while [[ $# -gt 0 ]]; do
  case "$1" in
    --env)
      ENVIRONMENT="$2"
      shift 2
      ;;
    --config)
      CONFIG="$2"
      shift 2
      ;;
    --prometheus-url)
      PROM_URL="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

case "$ENVIRONMENT" in
  dev)
    DEFAULT_CONFIG="ops/prometheus/prometheus.dev.yml"
    ;;
  prod)
    DEFAULT_CONFIG="ops/prometheus/prometheus.prod.yml"
    ;;
  *)
    echo "Invalid environment: $ENVIRONMENT" >&2
    exit 1
    ;;
esac

CONFIG=${CONFIG:-$DEFAULT_CONFIG}

require_cmd promtool
require_cmd curl
require_cmd python3

if [[ ! -f "$CONFIG" ]]; then
  echo "Configuration file not found: $CONFIG" >&2
  exit 1
fi

REPO_ROOT=$(git rev-parse --show-toplevel)
OUT_ROOT="$REPO_ROOT/out/obs_gatecheck/prometheus"
RUN_ID=$(python3 -c 'import uuid; print(uuid.uuid4())')
RUN_DIR="$OUT_ROOT/$RUN_ID"
LOG_DIR="$RUN_DIR/logs"
INSTANT_DIR="$RUN_DIR/queries/instant"
RANGE_DIR="$RUN_DIR/queries/range"
TELEMETRY_DIR="$RUN_DIR/telemetry"
mkdir -p "$LOG_DIR" "$INSTANT_DIR" "$RANGE_DIR" "$TELEMETRY_DIR"

log "OBS-3 run id: $RUN_ID"
log "Pre-flight: validating Prometheus config $CONFIG"
promtool check config "$CONFIG" | tee "$LOG_DIR/promtool_check.log"

log "Running rule tests"
promtool test rules ops/prometheus/tests/core.rules.test.yml | tee "$LOG_DIR/promtool_rules_test.log"

log "Checking Prometheus readiness at $PROM_URL/-/ready"
READINESS_FILE="$TELEMETRY_DIR/readiness.json"
if curl --fail --silent --show-error "$PROM_URL/-/ready" > "$READINESS_FILE"; then
  log "Prometheus readiness probe succeeded"
else
  log "Prometheus readiness probe failed; continuing with collected artifacts"
fi

log "Fetching target metadata"
curl --silent --show-error "$PROM_URL/api/v1/targets" > "$TELEMETRY_DIR/targets.json" || true
curl --silent --show-error "$PROM_URL/api/v1/rules" > "$TELEMETRY_DIR/rules.json" || true

now=$(date +%s)
start=$((now - 300))
step=${OBS3_RANGE_STEP:-30}

query_instant() {
  local expr="$1"
  local outfile="$2"
  curl --silent --show-error --get "$PROM_URL/api/v1/query" \
    --data-urlencode "query=$expr" > "$outfile"
}

query_range() {
  local expr="$1"
  local outfile="$2"
  curl --silent --show-error --get "$PROM_URL/api/v1/query_range" \
    --data-urlencode "query=$expr" \
    --data-urlencode "start=$start" \
    --data-urlencode "end=$now" \
    --data-urlencode "step=${step}" > "$outfile"
}

log "Collecting histogram and counter ranges"
query_range 'amm_op_latency_seconds_bucket' "$RANGE_DIR/amm_op_latency_seconds_bucket.json" || true
query_range 'amm_op_latency_seconds_sum' "$RANGE_DIR/amm_op_latency_seconds_sum.json" || true
query_range 'amm_op_latency_seconds_count' "$RANGE_DIR/amm_op_latency_seconds_count.json" || true
query_range 'amm_hook_invocations_total' "$RANGE_DIR/amm_hook_invocations_total.json" || true

log "Collecting recording rules snapshots"
query_instant 'ce:amm_op_latency_seconds:p75' "$INSTANT_DIR/latency_p75.json" || true
query_instant 'ce:amm_op_latency_seconds:p95' "$INSTANT_DIR/latency_p95.json" || true
query_instant 'ce:amm_op_latency_seconds:avg' "$INSTANT_DIR/latency_avg.json" || true
query_instant 'ce:amm_hook_invocations:throughput_5m' "$INSTANT_DIR/hooks_throughput.json" || true
query_instant 'ce:data_feature:max_5m' "$INSTANT_DIR/data_feature_max.json" || true

QUALITY_REPORT="$RUN_DIR/quality_report.json"
log "Running OBS-3 quality checks"
python3 "$REPO_ROOT/scripts/obs3_quality_checks.py" \
  --instant-dir "$INSTANT_DIR" \
  --range-dir "$RANGE_DIR" \
  --output "$QUALITY_REPORT" \
  --log "$LOG_DIR/quality_checks.log"

log "Generating evidence manifest"
python3 "$REPO_ROOT/scripts/obs3_hash_manifest.py" \
  --env "$ENVIRONMENT" \
  --run-id "$RUN_ID" \
  --evidence-dir "$RUN_DIR" \
  --output "$RUN_DIR/manifest.json"

python3 "$REPO_ROOT/scripts/obs3_verify_manifest.py" \
  --schema "$REPO_ROOT/ops/schemas/manifest.schema.json" \
  --manifest "$RUN_DIR/manifest.json"

log "Evidence ready at $RUN_DIR"
log "Artifacts:"
find "$RUN_DIR" -maxdepth 1 -type f -printf '  %f\n'

