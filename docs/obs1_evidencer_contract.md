# OBS-1 Evidencer Contract (`scripts/obs_evidencer.sh`)

## 1. Objetivo
O evidencer automatiza a coleta de provas exigidas pelo pacote OBS-1. Ele executa o binário `obs_demo`, coleta telemetria (métricas, spans e logs), calcula `sha256` de fontes relevantes e consolida um manifesto JSON canônico em `out/obs_gatecheck/evidence/obs1_sdk.json`. O processo é idempotente: rodadas subsequentes sobrescrevem artefatos anteriores e mantêm o diretório `out/obs_gatecheck/` pronto para anexar no PR.

## 2. Pré-requisitos
- `cargo` disponível no `PATH` (o binário é compilado e executado via `cargo run`).
- `curl` instalado quando `--prom` for utilizado (requisição ao endpoint `/metrics`).
- Porta `METRICS_HTTP_ADDR` livre (default `127.0.0.1:9464`).
- Ambiente local com dependências Rust já compiladas (o primeiro run pode gastar tempo em build).

## 3. Como executar
### 3.1 Execução padrão com Prometheus
```bash
scripts/obs_evidencer.sh --ops 8 --timeout-secs 30 --prom
```
### 3.2 Execução com exportação OTLP explícita
```bash
scripts/obs_evidencer.sh --ops 8 --timeout-secs 30 --otlp http://localhost:4317
```
### 3.3 Parâmetros e variáveis de ambiente
- `--ops N`: número de operações sintéticas (default `10`). O valor também é propagado via `-- "--ops" N` para o binário e pela variável `OBS_DEMO_OPS` (quando suportada).
- `--timeout-secs T`: janela para aguardar o `/metrics` responder HTTP 200.
- `--prom`: habilita `PROM_SCRAPE=on` e usa `METRICS_HTTP_ADDR` (default `127.0.0.1:9464`).
- `--otlp URL`: define `OTEL_EXPORTER_OTLP_ENDPOINT` caso exista um collector disponível.
- Variáveis resolvidas automaticamente quando não exportadas previamente:
  - `DEPLOY_ENV` → `dev` (padrão).
  - `OBSERVABILITY_LEVEL` → `full`.
  - `PROM_SCRAPE` → `on` ou `off` conforme flags.
  - `METRICS_HTTP_ADDR` → `127.0.0.1:9464`.

## 4. O que é validado
1. **Logs estruturados**: o arquivo `out/obs_gatecheck/logs/obs1_smoke.txt` precisa conter pelo menos uma linha JSON com os campos `ts`, `level`, `msg`, `service`, `env`, `version`, `op`, `trace_id` e `span_id`. A ausência desses campos aborta a execução.
2. **Traces**: a contagem de `trace_id` e `span_id` distintos é calculada a partir dos logs. O evidencer falha se não observar ao menos um valor único de cada.
3. **Métricas Prometheus** (quando `--prom`):
   - O script aguarda até `timeout-secs` por uma resposta 200 em `http://$METRICS_HTTP_ADDR/metrics`.
   - O conteúdo é salvo em `out/obs_gatecheck/logs/obs1_metrics_sample.txt`.
   - Exige pelo menos **3** linhas `amm_op_latency_seconds_bucket` com valor `> 0`.
   - Marca `hook_executions_total.present` como `true` ao detectar a métrica na amostra.
4. **Hashes de fonte**: calcula `sha256` dos arquivos de telemetria existentes listados no contrato (por exemplo `src/telemetry_cfg.rs`, `src/bin/obs_demo.rs`). Entradas ausentes são simplesmente ignoradas.
5. **Manifesto JSON**: gera `obs1_sdk.json` com timestamp UTC em RFC3339, identidade do serviço (`ce-amm`), indicadores de métricas/logs/traces, endpoint OTLP (ou `null`) e caminhos de artefatos.

## 5. Estrutura do manifesto
```json
{
  "timestamp_utc": "<RFC3339>",
  "service": { "name": "ce-amm", "version": "<resolved>", "env": "<dev|stg|prod>" },
  "prom_scrape": true,
  "metrics": {
    "amm_op_latency_seconds": { "buckets_nonzero": <int>, "sample_extract": "<string>" },
    "hook_executions_total": { "present": <bool> }
  },
  "traces": { "observed_trace_ids": <int>, "observed_span_ids": <int> },
  "logs": { "lines_json": <int>, "sample": "<line>" },
  "otlp": { "endpoint": "<url or null>" },
  "artifacts": {
    "smoke_log": "out/obs_gatecheck/logs/obs1_smoke.txt",
    "metrics_sample": "out/obs_gatecheck/logs/obs1_metrics_sample.txt"
  },
  "sources_sha256": [
    { "path": "src/telemetry_cfg.rs", "sha256": "..." },
    { "path": "src/bin/obs_demo.rs", "sha256": "..." }
  ]
}
```
Observações:
- Quando `--prom` não é usado, `prom_scrape=false`, `metrics_sample` fica `null`, e `buckets_nonzero=0`.
- `sources_sha256` contém apenas arquivos que existem no repositório.

## 6. Diagnóstico e resolução de falhas
| Sintoma | Causa provável | Mitigação |
| --- | --- | --- |
| `failed to scrape /metrics` | Porta ocupada, exporter não iniciou ou `curl` ausente | Verifique `METRICS_HTTP_ADDR`, libere a porta, instale `curl` e repita. Confira logs no `obs1_smoke.txt`. |
| `obs_demo exited with code ...` | Compilação falhou ou execução panicking | Rode `cargo run --bin obs_demo` manualmente para capturar o erro, corrija dependências e volte a executar o evidencer. |
| `no structured log lines` | Telemetria sem spans/logs ou filtros `RUST_LOG` muito restritivos | Ajuste configurações de observabilidade (ex.: `OBSERVABILITY_LEVEL=full`) e valide novamente. |
| `expected at least 3 non-zero buckets` | Operações insuficientes ou métrica não instrumentada | Aumente `--ops`, garanta que `PROM_SCRAPE=on` e que o exporter esteja ativo. |

## 7. Reprodutibilidade e anexos de PR
- Todos os artefatos ficam em `out/obs_gatecheck/` e são sobrescritos em execuções posteriores.
- Anexe `obs1_sdk.json`, `logs/obs1_smoke.txt` e (quando aplicável) `logs/obs1_metrics_sample.txt` ao PR para revisão.
- Mantenha `scripts/obs_evidencer.sh` versionado junto com o manifesto produzido na execução de validação.

## 8. Boas práticas adicionais
- Rodar `shellcheck scripts/obs_evidencer.sh` quando disponível para garantir portabilidade.
- Antes de commitar, execute `tests/obs_evidencer_tests.sh` para validar rapidamente o fluxo end-to-end.
- Use `--timeout-secs` maior em ambientes lentos (ex.: CI com cold start de exporters).
