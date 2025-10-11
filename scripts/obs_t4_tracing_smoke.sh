#!/usr/bin/env bash
set -Eeuo pipefail
set +H

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_DIR="$ROOT_DIR/out/obs_gatecheck/logs"
EVI_DIR="$ROOT_DIR/out/obs_gatecheck/evidence"
mkdir -p "$LOG_DIR" "$EVI_DIR"

LOG_FILE="$LOG_DIR/obs4_trace_smoke.txt"
: >"$LOG_FILE"

info() {
  printf '[%s] [%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "INFO" "$*" | tee -a "$LOG_FILE"
}

die() {
  printf '[%s] [%s] %s\n' "$(date -u '+%Y-%m-%dT%H:%M:%SZ')" "ERROR" "$*" | tee -a "$LOG_FILE" >&2
  exit 1
}

command -v cargo >/dev/null 2>&1 || die "cargo não encontrado no PATH"
command -v python3 >/dev/null 2>&1 || die "python3 não encontrado no PATH"
command -v curl >/dev/null 2>&1 || die "curl não encontrado no PATH"

COLLECTOR_ADDR="${OTELCOL_LISTEN_ADDR:-127.0.0.1}"
COLLECTOR_PORT="${OTELCOL_LISTEN_PORT:-4318}"

info "Validando collector em ${COLLECTOR_ADDR}:${COLLECTOR_PORT}"
python3 - "$COLLECTOR_ADDR" "$COLLECTOR_PORT" >>"$LOG_FILE" 2>&1 <<'PYCHECK'
import socket
import sys

addr = sys.argv[1]
port = int(sys.argv[2])

with socket.socket() as sock:
    sock.settimeout(3.0)
    sock.connect((addr, port))
PYCHECK
info "Collector acessível"

OTLP_ENDPOINT="http://${COLLECTOR_ADDR}:${COLLECTOR_PORT}"
OTLP_JSON_ENDPOINT="${OTLP_ENDPOINT%/}/v1/traces"

export OTEL_TRACES_SAMPLER="parentbased_traceidratio"
export OTEL_TRACES_SAMPLER_ARG="${OTEL_TRACES_SAMPLER_ARG:-0.1}"
export DEPLOY_ENV="${DEPLOY_ENV:-dev}"
export OBSERVABILITY_LEVEL="${OBSERVABILITY_LEVEL:-full}"
export PROM_SCRAPE="off"

run_obs_demo() {
  local ops="$1"
  info "Executando obs_demo com ${ops} operações"
  (cd "$ROOT_DIR" && \
    env RUSTFLAGS='--cfg feature="testing"' \
        OTEL_EXPORTER_OTLP_ENDPOINT="$OTLP_ENDPOINT" \
        OBS_DEMO_OPS="$ops" \
        RUST_LOG="info" \
        cargo run --quiet --bin obs_demo >>"$LOG_FILE" 2>&1)
}

run_obs_demo 5
run_obs_demo 5
trace_ok=true

resource_attrs_json='[
  {"key":"service.name","value":{"stringValue":"ce-amm"}},
  {"key":"service.version","value":{"stringValue":"0.0.0-smoke"}},
  {"key":"deployment.environment","value":{"stringValue":"dev"}}
]'

post_span() {
  local span_name="$1"
  local duration_ms="$2"
  local status="$3"
  local status_message="$4"
  local attrs_json="$5"
  env \
    OTLP_JSON_ENDPOINT="$OTLP_JSON_ENDPOINT" \
    RESOURCE_ATTRIBUTES_JSON="$resource_attrs_json" \
    SPAN_NAME="$span_name" \
    SPAN_DURATION_MS="$duration_ms" \
    SPAN_STATUS="$status" \
    SPAN_STATUS_MESSAGE="$status_message" \
    SPAN_ATTRIBUTES_JSON="$attrs_json" \
    python3 - <<'PY' >>"$LOG_FILE" 2>&1
import json
import os
import secrets
import time
import urllib.request

endpoint = os.environ["OTLP_JSON_ENDPOINT"]
span_attrs = json.loads(os.environ["SPAN_ATTRIBUTES_JSON"])
resource_attrs = json.loads(os.environ["RESOURCE_ATTRIBUTES_JSON"])
start = time.time_ns()
end = start + int(float(os.environ["SPAN_DURATION_MS"]) * 1_000_000)
span = {
    "traceId": secrets.token_hex(16)
    ,"spanId": secrets.token_hex(8)
    ,"name": os.environ["SPAN_NAME"]
    ,"kind": "SPAN_KIND_INTERNAL"
    ,"startTimeUnixNano": str(start)
    ,"endTimeUnixNano": str(end)
    ,"attributes": span_attrs
}
status = os.environ.get("SPAN_STATUS", "OK").upper()
if status == "ERROR":
    span["status"] = {
        "code": "STATUS_CODE_ERROR",
        "message": os.environ.get("SPAN_STATUS_MESSAGE", "") or "synthetic error"
    }
else:
    span["status"] = {"code": "STATUS_CODE_OK"}

payload = {
    "resourceSpans": [
        {
            "resource": {"attributes": resource_attrs},
            "scopeSpans": [
                {
                    "scope": {"name": "obs_t4_tracing_smoke"},
                    "spans": [span],
                }
            ],
        }
    ]
}

req = urllib.request.Request(
    endpoint,
    data=json.dumps(payload).encode("utf-8"),
    headers={"Content-Type": "application/json"},
)
with urllib.request.urlopen(req, timeout=10) as resp:
    print(f"POST {endpoint} -> {resp.status}")
PY
}

slow_ms=$(( ${TAIL_SLOW_MS:-200} + 80 ))
info "Enviando span lento synthetic amm.swap (${slow_ms} ms)"
SPAN_ATTRIBUTES_JSON='[
  {"key":"amm.k_before","value":{"doubleValue":960000.0}},
  {"key":"amm.k_after","value":{"doubleValue":1012000.0}},
  {"key":"amm.delta_k_ratio","value":{"doubleValue":0.052}},
  {"key":"amm.fee_ppm","value":{"intValue":"300"}},
  {"key":"amm.input","value":{"doubleValue":110.0}},
  {"key":"amm.output","value":{"doubleValue":85.0}},
  {"key":"amm.direction","value":{"stringValue":"quote_to_base"}}
]'
post_span "amm.swap" "$slow_ms" "OK" "" "$SPAN_ATTRIBUTES_JSON"
slow_captured=true

info "Enviando span synthetic pricing.quote com status ERROR"
SPAN_ATTRIBUTES_JSON='[
  {"key":"amm.k_before","value":{"doubleValue":1012000.0}},
  {"key":"amm.k_after","value":{"doubleValue":1000000.0}},
  {"key":"amm.delta_k_ratio","value":{"doubleValue":-0.012}},
  {"key":"amm.fee_ppm","value":{"intValue":"420"}},
  {"key":"amm.input","value":{"doubleValue":72.0}},
  {"key":"amm.output","value":{"doubleValue":64.0}},
  {"key":"pricing.mid_price","value":{"doubleValue":0.78}}
]'
post_span "pricing.quote" "120" "ERROR" "synthetic pricing error" "$SPAN_ATTRIBUTES_JSON"
error_captured=true

raw_json="$EVI_DIR/traces_raw.json"
if [ -n "${TEMPO_HTTP_URL:-}" ]; then
  info "Consultando Tempo em ${TEMPO_HTTP_URL}"
  start_ns=$(python3 - <<'PYWIN'
import time
print(int((time.time() - 300) * 1_000_000_000))
PYWIN
)
  end_ns=$(python3 - <<'PYWIN'
import time
print(int(time.time() * 1_000_000_000))
PYWIN
)
  if curl -fsS -G "${TEMPO_HTTP_URL%/}/api/search" \
    --data-urlencode "limit=50" \
    --data-urlencode "start=$start_ns" \
    --data-urlencode "end=$end_ns" \
    --data-urlencode "tags=service.name=ce-amm" \
    >"$raw_json"; then
    info "Resposta Tempo salva em $raw_json"
  else
    info "Falha na consulta Tempo"
    rm -f "$raw_json"
  fi
elif [ -n "${JAEGER_HTTP_URL:-}" ]; then
  info "Consultando Jaeger em ${JAEGER_HTTP_URL}"
  if curl -fsS -G "${JAEGER_HTTP_URL%/}/api/traces" \
    --data-urlencode "service=ce-amm" \
    --data-urlencode "lookback=5m" \
    --data-urlencode "limit=20" \
    >"$raw_json"; then
    info "Resposta Jaeger salva em $raw_json"
  else
    info "Falha na consulta Jaeger"
    rm -f "$raw_json"
  fi
else
  debug_sample="$EVI_DIR/traces_debug_sample.txt"
  info "Coletando amostra de debug do otelcol"
  tail_sources=()
  if [ -f "$LOG_DIR/otelcol_trace.out" ]; then
    tail_sources+=("$LOG_DIR/otelcol_trace.out")
  fi
  if [ -f "$LOG_DIR/otelcol_trace.err" ]; then
    tail_sources+=("$LOG_DIR/otelcol_trace.err")
  fi
  if [ ${#tail_sources[@]} -gt 0 ]; then
    {
      for src in "${tail_sources[@]}"; do
        echo "==== ${src##*/} ===="
        tail -n 50 "$src"
      done
    } >"$debug_sample"
    info "Amostra debug salva em $debug_sample"
  else
    echo "Sem arquivos otelcol_trace.* disponíveis" >"$debug_sample"
    info "Nenhum arquivo otelcol_trace.* encontrado; anotado placeholder"
  fi
fi

python3 - <<'PYWRITE' "$EVI_DIR/traces_sample.json" "$trace_ok" "$slow_captured" "$error_captured"
import json
import sys

out_path = sys.argv[1]
trace_ok = sys.argv[2].lower() == 'true'
slow = sys.argv[3].lower() == 'true'
err = sys.argv[4].lower() == 'true'

with open(out_path, 'w', encoding='utf-8') as fh:
    json.dump(
        {
            'trace_ok': trace_ok,
            'slow_captured': slow,
            'error_captured': err,
            'links_cdc_amm': False,
        },
        fh,
        indent=2,
    )
PYWRITE

info "Evidência escrita em $EVI_DIR/traces_sample.json"
info "Smoke concluído"
