# OBS-4 • Ambiente de tracing

O fluxo de smoke `obs_t4_tracing_smoke.sh` valida rapidamente a ingestão de spans no collector local e garante evidências mínimas para auditoria.

## Pré-requisitos
- Collector OTLP ativo (Thread 02) ouvindo em `${OTELCOL_LISTEN_ADDR:-127.0.0.1}:${OTELCOL_LISTEN_PORT:-4318}`.
- `cargo`, `python3` e `curl` instalados.
- Ambiente virtual Python opcional (o script usa apenas a stdlib).

## Execução do smoke
```bash
bash scripts/obs_t4_tracing_smoke.sh
python3 scripts/obs4_tracing_acceptance.py
```

Use `out/obs_gatecheck/logs/obs4_thread07_run.txt` para capturar stdout/stderr consolidado quando rodar os dois comandos em sequência:
```bash
{
  bash scripts/obs_t4_tracing_smoke.sh
  python3 scripts/obs4_tracing_acceptance.py
} &> out/obs_gatecheck/logs/obs4_thread07_run.txt
```

## Evidências geradas
- `out/obs_gatecheck/logs/obs4_trace_smoke.txt`: log detalhado da execução (inclui IDs de trace/span).
- `out/obs_gatecheck/evidence/traces_sample.json`: flags canônicas (`trace_ok`, `slow_captured`, `error_captured`, `links_cdc_amm`).
- `out/obs_gatecheck/evidence/traces_raw.json`: dump bruto do backend (Tempo/Jaeger) quando disponível.
- `out/obs_gatecheck/evidence/traces_debug_sample.txt`: fallback com recorte `otelcol_trace.out/err` caso o backend HTTP não esteja configurado.

A aceitação automática (`scripts/obs4_tracing_acceptance.py`) garante que o JSON de evidência contenha apenas booleanos válidos e, quando existir `traces_raw.json`, verifica se ao menos um trace apresenta `status=ERROR`.
