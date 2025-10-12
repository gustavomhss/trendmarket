set -Eeuo pipefail
: "${ROOT:=$PWD}"
: "${OUT:=out/obs_gatecheck}"
: "${TS:=$(date +%Y-%m-%dT%H:%M:%S%z)}"
: "${OTELCOL_LISTEN_ADDR:=127.0.0.1}"
: "${OTELCOL_BIN:=$(command -v otelcol-contrib || command -v otelcol || true)}"
: "${CFG:=ops/otel/collector-dev.prom.yaml}"
mkdir -p "$OUT"/{logs,evidence} "$ROOT/out"
[ -z "$OTELCOL_BIN" ] && echo OTELCOL_NOT_FOUND && exit 127
[ ! -f "$CFG" ] && echo CONFIG_NOT_FOUND && exit 2
pkill -f 'otelcol' >/dev/null 2>&1 || true
nohup "$OTELCOL_BIN" --config "$CFG" > "$OUT/logs/collector.txt" 2>&1 &
for i in $(seq 1 30); do sleep 0.5; H1="$(curl -sf localhost:13133/healthz || true)"; [ "$H1" = "Server available" ] && break; done
CM="$(curl -sf localhost:8888/metrics || true)"
TMRC="$(curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:13200 || true)"
[ "$TMRC" = "200" ] || [ "$TMRC" = "404" ] && TM_OK=1 || TM_OK=0
SENT="$(printf "%s\n" "$CM" | awk '/^otelcol_exporter_sent_spans(\{|[[:space:]])/{n=NF;v=$n;if(n>=2&&($n~/^[0-9]+$/)&&($(n-1)~/^[0-9.]+$/))v=$(n-1);s+=v+0}END{print s+0}')"
[ -z "$SENT" ] && SENT=0
if [ "$SENT" -lt 1 ]; then
  if [ -x target/debug/obs_demo ]; then RUST_LOG=info target/debug/obs_demo >/dev/null 2>&1 || true; else RUST_LOG=info cargo run --features obs --bin obs_demo -q >/dev/null 2>&1 || true; fi
  sleep 1
  CM="$(curl -sf localhost:8888/metrics || true)"
  SENT="$(printf "%s\n" "$CM" | awk '/^otelcol_exporter_sent_spans(\{|[[:space:]])/{n=NF;v=$n;if(n>=2&&($n~/^[0-9]+$/)&&($(n-1)~/^[0-9.]+$/))v=$(n-1);s+=v+0}END{print s+0}')"
  [ -z "$SENT" ] && SENT=0
fi
ACC=1
[ "${H1:-}" != "Server available" ] && ACC=0
! grep -qE '^otelcol_' <<<"$CM" && ACC=0
[ "$TM_OK" -ne 1 ] && ACC=0
STAT=$([ "$ACC" -eq 1 ] && [ "$SENT" -ge 1 ] && echo GATECHECK_OK || echo GATECHECK_WARN)
printf "%s\n" "$CM" > "$OUT/evidence/metrics.txt"
printf "Tempo_UI=%s\n" "$TMRC" > "$OUT/evidence/summary_tempo.txt"
cat > "$OUT/JIRA_COMMENT_GATECHECK.md" <<EOF
### OBS-4 — Gatecheck Collector (Tempo/Jaeger)
- Data: ${TS}
- Endpoint de telemetria: ${OTELCOL_LISTEN_ADDR}:4318
- Resultado: ${STAT}
- accepted=${ACC}, sent_tempo=${SENT}

Artefatos:
- logs: ${OUT}/logs/collector.txt
- métricas: ${OUT}/evidence/metrics.txt
- resumo tempo: ${OUT}/evidence/summary_tempo.txt

UI do Tempo: http://127.0.0.1:13200
EOF
TSB="$(date +%Y%m%d-%H%M%S)"
BUNDLE="$ROOT/out/obs_gatecheck_bundle_${TSB}.zip"
( cd "$ROOT/out" && zip -qr "$(basename "$BUNDLE}")" "obs_gatecheck" )
shasum -a 256 "$BUNDLE" | tee "$OUT/evidence/bundle.sha256.txt"
printf "STATUS=%s ACC=%s SENT=%s TM_OK=%s\nBUNDLE=%s\n" "$STAT" "$ACC" "$SENT" "$TM_OK" "$BUNDLE"
