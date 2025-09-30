#!/usr/bin/env bash
set -Eeuo pipefail
export LC_ALL=C
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"; EVI="$OUT/evidence/metrics"; DOC="$OUT/docs"
mkdir -p "$LOG" "$EVI" "$DOC"

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG/t6_run.log"; }

note "Higiene: conflitos e placeholders"
if grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . >/dev/null; then echo "ERRO: Conflitos detectados" | tee -a "$LOG/t6_run.log"; exit 2; fi
if grep -RInE '\\.\\.\\.|TBD|FIXME' -n src/telemetry.rs src/bin/telemetry_smoke.rs observability 2>/dev/null; then echo "ERRO: Placeholder detectado" | tee -a "$LOG/t6_run.log"; exit 3; fi

note "Build do binário de smoke (feature obs)"
ADDR=${AMM_METRICS_ADDR:-127.0.0.1:9464}
RUST_LOG=${RUST_LOG:-info}
set -o pipefail
AMM_METRICS_ADDR="$ADDR" cargo run --features obs --bin telemetry_smoke 2>&1 | tee "$LOG/telemetry_smoke_run.txt" || true

note "Scrape do endpoint /metrics"
python3 - <<PY 2>&1 | tee "$LOG/t6_scrape.txt"
import os, sys, socket, time, urllib.request, json, pathlib
addr=os.getenv('AMM_METRICS_ADDR','127.0.0.1:9464')
url=f"http://{addr}/metrics"
for i in range(15):
    try:
        with urllib.request.urlopen(url, timeout=1.0) as r:
            body=r.read().decode('utf-8','ignore')
            pathlib.Path('out/orr_gatecheck/evidence/metrics/smoke.txt').write_text(body, encoding='utf-8')
            break
    except Exception:
        time.sleep(0.2)
else:
    print('ERRO: não foi possível coletar /metrics'); sys.exit(4)

# Registro de porta
pathlib.Path('out/orr_gatecheck/evidence/metrics/ports.json').write_text(json.dumps({"prometheus_http":addr}, indent=2), encoding='utf-8')
PY

note "Validações de conteúdo"
SMK="$OUT/evidence/metrics/smoke.txt"
req=( "amm_swaps_total" "amm_liquidity_ops_total" "amm_error_total" "amm_swap_latency_ms" )
for k in "${req[@]}"; do
  grep -q "$k" "$SMK" || { echo "ERRO: métrica obrigatória ausente: $k" | tee -a "$LOG/t6_run.log"; exit 5; }
done

note "T6 concluída"
