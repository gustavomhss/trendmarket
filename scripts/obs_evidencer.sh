#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

usage() {
    cat <<'USAGE'
Usage: scripts/obs_evidencer.sh [options]

Options:
  --ops N             Number of synthetic ops to trigger in obs_demo (default: 10)
  --timeout-secs T    Timeout in seconds for readiness checks (default: 30)
  --prom              Enable Prometheus scraping via PROM_SCRAPE=on (default disabled)
  --otlp URL          Configure OTEL_EXPORTER_OTLP_ENDPOINT (default unset)
  --help              Show this help message
USAGE
}

OPS=10
TIMEOUT_SECS=30
PROM_ENABLED=false
METRICS_HTTP_ADDR_DEFAULT="127.0.0.1:9464"
OTLP_ENDPOINT=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --ops)
            [ "${2-}" ] || { echo "missing value for --ops" >&2; exit 2; }
            OPS="$2"
            shift 2
            ;;
        --timeout-secs)
            [ "${2-}" ] || { echo "missing value for --timeout-secs" >&2; exit 2; }
            TIMEOUT_SECS="$2"
            shift 2
            ;;
        --prom)
            PROM_ENABLED=true
            shift 1
            ;;
        --otlp)
            [ "${2-}" ] || { echo "missing value for --otlp" >&2; exit 2; }
            OTLP_ENDPOINT="$2"
            shift 2
            ;;
        --help)
            usage
            exit 0
            ;;
        *)
            echo "unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

if ! [[ "$OPS" =~ ^[0-9]+$ ]]; then
    echo "--ops must be an integer" >&2
    exit 2
fi

if ! [[ "$TIMEOUT_SECS" =~ ^[0-9]+$ ]]; then
    echo "--timeout-secs must be an integer" >&2
    exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required to run obs_demo" >&2
    exit 1
fi

if $PROM_ENABLED && ! command -v curl >/dev/null 2>&1; then
    echo "curl is required when --prom is enabled" >&2
    exit 1
fi

mkdir -p out/obs_gatecheck/evidence out/obs_gatecheck/logs

SMOKE_LOG="out/obs_gatecheck/logs/obs1_smoke.txt"
METRICS_SAMPLE="out/obs_gatecheck/logs/obs1_metrics_sample.txt"
MANIFEST_PATH="out/obs_gatecheck/evidence/obs1_sdk.json"

: >"$SMOKE_LOG"
if $PROM_ENABLED; then
    : >"$METRICS_SAMPLE"
fi

DEPLOY_ENV_VALUE=${DEPLOY_ENV:-dev}
OBS_LEVEL_VALUE=${OBSERVABILITY_LEVEL:-full}
PROM_SCRAPE_VALUE="off"
METRICS_HTTP_ADDR_VALUE=${METRICS_HTTP_ADDR:-$METRICS_HTTP_ADDR_DEFAULT}
if $PROM_ENABLED; then
    PROM_SCRAPE_VALUE="on"
fi

export DEPLOY_ENV="$DEPLOY_ENV_VALUE"
export OBSERVABILITY_LEVEL="$OBS_LEVEL_VALUE"
export PROM_SCRAPE="$PROM_SCRAPE_VALUE"
export METRICS_HTTP_ADDR="$METRICS_HTTP_ADDR_VALUE"
export OBS_DEMO_OPS="$OPS"
if [ -n "$OTLP_ENDPOINT" ]; then
    export OTEL_EXPORTER_OTLP_ENDPOINT="$OTLP_ENDPOINT"
else
    unset OTEL_EXPORTER_OTLP_ENDPOINT 2>/dev/null || true
fi

OBS_CMD=(cargo run --quiet --bin obs_demo -- "--ops" "$OPS")

set +e
"${OBS_CMD[@]}" >"$SMOKE_LOG" 2>&1 &
OBS_PID=$!
set -e

OBS_EXIT=0

if $PROM_ENABLED; then
    METRICS_READY=false
    START_TS=$(date +%s)
    while true; do
        if curl --fail --silent --show-error "http://$METRICS_HTTP_ADDR_VALUE/metrics" >"$METRICS_SAMPLE" 2>>"$SMOKE_LOG"; then
            METRICS_READY=true
            break
        fi

        NOW_TS=$(date +%s)
        if [ $((NOW_TS - START_TS)) -ge "$TIMEOUT_SECS" ]; then
            break
        fi

        if ! kill -0 "$OBS_PID" 2>/dev/null; then
            # process exited; attempt one final scrape before giving up
            if curl --fail --silent --show-error "http://$METRICS_HTTP_ADDR_VALUE/metrics" >"$METRICS_SAMPLE" 2>>"$SMOKE_LOG"; then
                METRICS_READY=true
            fi
            break
        fi

        sleep 1
    done

    if [ "$METRICS_READY" != true ]; then
        wait "$OBS_PID" || OBS_EXIT=$?
        echo "failed to scrape /metrics within ${TIMEOUT_SECS}s" >&2
        exit 1
    fi
fi

wait "$OBS_PID" || OBS_EXIT=$?
if [ "$OBS_EXIT" -ne 0 ]; then
    echo "obs_demo exited with code $OBS_EXIT" >&2
    exit "$OBS_EXIT"
fi

if [ ! -s "$SMOKE_LOG" ]; then
    echo "obs_demo produced no logs" >&2
    exit 1
fi

if $PROM_ENABLED && [ ! -s "$METRICS_SAMPLE" ]; then
    echo "metrics sample file is empty" >&2
    exit 1
fi

TIMESTAMP_UTC=$(python3 - <<'PY_TS'
import datetime
print(
    datetime.datetime.now(datetime.timezone.utc)
    .replace(microsecond=0)
    .isoformat()
    .replace('+00:00', 'Z')
)
PY_TS
)

SERVICE_VERSION=$(awk -F '"' '/^version\s*=\s*"/ {print $2; exit}' Cargo.toml)
if [ -z "$SERVICE_VERSION" ]; then
    SERVICE_VERSION="unknown"
fi

METRICS_BUCKETS_NONZERO=0
METRICS_SAMPLE_EXTRACT=""
HOOK_PRESENT=false
if $PROM_ENABLED; then
    metrics_analysis=$(python3 - "$METRICS_SAMPLE" <<'PY_METRICS'
import sys
from pathlib import Path

path = Path(sys.argv[1])
count = 0
sample = ""
hook_present = False

if path.exists():
    with path.open('r', encoding='utf-8', errors='replace') as fh:
        for line in fh:
            stripped = line.strip()
            if stripped.startswith('amm_op_latency_seconds_bucket'):
                parts = stripped.split()
                if len(parts) >= 2:
                    try:
                        value = float(parts[-1])
                    except ValueError:
                        continue
                    if value > 0:
                        count += 1
                        if not sample:
                            sample = stripped
            elif stripped.startswith('hook_executions_total'):
                hook_present = True

print(count)
print(sample)
print('true' if hook_present else 'false')
PY_METRICS
)
    IFS=$'\n' set -- $metrics_analysis
    METRICS_BUCKETS_NONZERO=${1-0}
    METRICS_SAMPLE_EXTRACT=${2-}
    HOOK_PRESENT=${3-false}

    if [ -z "$METRICS_BUCKETS_NONZERO" ]; then
        METRICS_BUCKETS_NONZERO=0
    fi

    if [ "$METRICS_BUCKETS_NONZERO" -lt 3 ]; then
        echo "expected at least 3 non-zero buckets for amm_op_latency_seconds" >&2
        exit 1
    fi
else
    METRICS_BUCKETS_NONZERO=0
    METRICS_SAMPLE_EXTRACT=""
    HOOK_PRESENT=false
fi

logs_analysis=$(python3 - "$SMOKE_LOG" <<'PY_LOGS'
import json
import sys
from pathlib import Path

log_path = Path(sys.argv[1])
trace_ids = set()
span_ids = set()
valid_lines = []
sample_line = ""

with log_path.open('r', encoding='utf-8', errors='replace') as fh:
    for line in fh:
        stripped = line.strip()
        if not stripped:
            continue
        try:
            data = json.loads(stripped)
        except json.JSONDecodeError:
            continue
        required = {'ts', 'level', 'msg', 'service', 'env', 'version', 'op', 'trace_id', 'span_id'}
        if not required.issubset(data.keys()):
            continue
        valid_lines.append(stripped)
        if not sample_line:
            sample_line = stripped
        trace_ids.add(data['trace_id'])
        span_ids.add(data['span_id'])

print(len(trace_ids))
print(len(span_ids))
print(len(valid_lines))
print(sample_line)
PY_LOGS
)
IFS=$'\n' set -- $logs_analysis
TRACE_COUNT=${1-0}
SPAN_COUNT=${2-0}
LOG_JSON_COUNT=${3-0}
LOG_SAMPLE=${4-}

if [ -z "$LOG_SAMPLE" ]; then
    echo "no structured log lines with trace/span/op detected" >&2
    exit 1
fi

if [ "$TRACE_COUNT" -lt 1 ] || [ "$SPAN_COUNT" -lt 1 ]; then
    echo "expected at least one trace/span in logs" >&2
    exit 1
fi

if [ "$LOG_JSON_COUNT" -lt 1 ]; then
    echo "expected at least one structured log line" >&2
    exit 1
fi

SOURCE_FILES=(
    "src/telemetry_cfg.rs"
    "src/telemetry_identity.rs"
    "src/telemetry_trace.rs"
    "src/telemetry_metrics_otlp.rs"
    "src/telemetry_metrics_prom.rs"
    "src/telemetry_logs.rs"
    "src/telemetry_instruments.rs"
    "src/telemetry_latency.rs"
    "src/telemetry_spans_amm.rs"
    "src/telemetry_spans_cdc.rs"
    "src/bin/obs_demo.rs"
)

SOURCE_SERIALIZED=""
for path in "${SOURCE_FILES[@]}"; do
    if [ -f "$path" ]; then
        sha=$(sha256sum "$path" | awk '{print $1}')
        entry="$path::$sha"
        if [ -z "$SOURCE_SERIALIZED" ]; then
            SOURCE_SERIALIZED="$entry"
        else
            SOURCE_SERIALIZED="$SOURCE_SERIALIZED;;$entry"
        fi
    fi
done

PROM_BOOL="false"
if $PROM_ENABLED; then
    PROM_BOOL="true"
fi

METRICS_SAMPLE_PATH=""
if $PROM_ENABLED; then
    METRICS_SAMPLE_PATH="$METRICS_SAMPLE"
fi

export TIMESTAMP="$TIMESTAMP_UTC"
export SERVICE_VERSION
export DEPLOY_ENV_VALUE
export PROM_BOOL
export METRICS_BUCKETS_NONZERO
export METRICS_SAMPLE_EXTRACT
export HOOK_PRESENT
export TRACE_COUNT
export SPAN_COUNT
export LOG_JSON_COUNT
export LOG_SAMPLE
export OTLP_ENDPOINT
export SMOKE_LOG_PATH="$SMOKE_LOG"
export METRICS_SAMPLE_PATH
export SOURCE_SERIALIZED

python3 - "$MANIFEST_PATH" <<'PY_MANIFEST'
import json
import os
import sys
from pathlib import Path

manifest = {
    "timestamp_utc": os.environ.get("TIMESTAMP", ""),
    "service": {
        "name": "ce-amm",
        "version": os.environ.get("SERVICE_VERSION", "unknown"),
        "env": os.environ.get("DEPLOY_ENV_VALUE", "dev"),
    },
    "prom_scrape": os.environ.get("PROM_BOOL", "false").lower() == "true",
    "metrics": {
        "amm_op_latency_seconds": {
            "buckets_nonzero": int(os.environ.get("METRICS_BUCKETS_NONZERO", "0") or 0),
            "sample_extract": os.environ.get("METRICS_SAMPLE_EXTRACT", ""),
        },
        "hook_executions_total": {
            "present": os.environ.get("HOOK_PRESENT", "false").lower() == "true",
        },
    },
    "traces": {
        "observed_trace_ids": int(os.environ.get("TRACE_COUNT", "0") or 0),
        "observed_span_ids": int(os.environ.get("SPAN_COUNT", "0") or 0),
    },
    "logs": {
        "lines_json": int(os.environ.get("LOG_JSON_COUNT", "0") or 0),
        "sample": os.environ.get("LOG_SAMPLE", ""),
    },
    "otlp": {
        "endpoint": os.environ.get("OTLP_ENDPOINT") or None,
    },
    "artifacts": {
        "smoke_log": os.environ.get("SMOKE_LOG_PATH", ""),
        "metrics_sample": os.environ.get("METRICS_SAMPLE_PATH") or None,
    },
    "sources_sha256": [],
}

source_serialized = os.environ.get("SOURCE_SERIALIZED", "")
if source_serialized:
    for chunk in source_serialized.split(';;'):
        if '::' not in chunk:
            continue
        path, sha = chunk.split('::', 1)
        manifest["sources_sha256"].append({"path": path, "sha256": sha})

manifest_path = Path(sys.argv[1])
manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
PY_MANIFEST

cat <<EOF_MSG
obs_evidencer completed successfully.
  smoke log: $SMOKE_LOG
  manifest: $MANIFEST_PATH
EOF_MSG
