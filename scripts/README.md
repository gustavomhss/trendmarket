# OBS-3 Thread 4 Runner — Prometheus Evidence Scraper

This runner orchestrates the observability evidence workflow for OBS-3 Thread 4. It validates Prometheus configuration, optionally boots a local Prometheus instance bound to loopback, waits for readiness, collects HTTP API evidence, and performs minimal dataset checks before delegating to downstream threads when available.

## Pré-requisitos

Execute the script on Linux ou macOS com os binários abaixo disponíveis no `PATH` (ou informe caminhos via flags):

- `prometheus`
- `promtool`
- `curl`
- `jq`
- `grep`
- `awk`
- `sed`
- `date`

Scripts opcionais requerem `python3` quando presentes.

## Uso

### Ambiente de desenvolvimento

Inicie uma coleta completa usando a configuração padrão de desenvolvimento:

```sh
./scripts/obs_t3_prom_scrape_run.sh --config ops/prometheus/prometheus.dev.yml
```

### Ambiente de produção / Prometheus já ativo

Se um Prometheus confiável já estiver rodando (ex.: `ops/prometheus/prometheus.prod.yml`), apenas colete as evidências sem reiniciar o serviço:

```sh
./scripts/obs_t3_prom_scrape_run.sh \
  --config ops/prometheus/prometheus.prod.yml \
  --addr 127.0.0.1:9090 \
  --skip-start \
  --no-stop
```

> **Importante:** O runner sempre força loopback (`127.0.0.1`/`::1`). Nunca exponha o Prometheus publicamente.

### Flags relevantes

- `--out <dir>`: diretório raiz para artefatos (`out/obs_gatecheck` padrão).
- `--retention <dur>`: retenção TSDB quando Prometheus é iniciado localmente (`7d`).
- `--ready-attempts` / `--ready-sleep`: controle do polling de readiness (padrões `30` tentativas e `1s`).
- `--skip-lint`: desativa `promtool check`. Útil para investigações rápidas.
- `--adhoc-only`: evita consultas `ce:*`, limitando-se aos quantis ad-hoc (latência).
- `--skip-start` / `--no-stop`: reutiliza processos externos existentes.
- `--prometheus-bin`, `--promtool-bin`, `--curl-bin`, `--jq-bin`: caminhos explícitos para binários.

## Artefatos gerados

Todos os arquivos ficam em `<out>/obs_gatecheck/{logs,evidence}` (ou no diretório fornecido via `--out`):

- `logs/prom_check.txt`
- `logs/prometheus.txt` (quando o runner inicia o Prometheus)
- `logs/prom.pid` (quando o runner inicia o Prometheus)
- `evidence/prom_targets.json`
- `evidence/prom_rules.json`
- `evidence/prom_up.json`
- `evidence/prom_p75_rec.json`
- `evidence/prom_p95_rec.json`
- `evidence/prom_p75_adhoc.json`
- `evidence/prom_p95_adhoc.json`
- `evidence/prom_series.json`

## Exit codes

| Código | Significado |
| ------ | ----------- |
| 0 | Execução concluída com sucesso. |
| 2 | Falha nos checks do `promtool`. |
| 3 | Prometheus não ficou pronto dentro da janela configurada. |
| 5 | Não foi possível gravar alguma evidência obrigatória. |
| 6 | Falha na validação mínima de datasets (evidências insuficientes). |
| 7 | Dependência obrigatória ausente (binário ou `python3` para integrações). |
| 10+ | Exit codes propagados de scripts opcionais (Threads 5–7). |

## Fail-fast de coleta

O runner valida automaticamente as evidências capturadas. Ele falha (`exit 6`) quando:

- A consulta `up` não retorna status `success` com pelo menos uma série `value[1] == 1`.
- Nenhum dataset de latência está disponível (`ce:*` ou quantis ad-hoc) com resultados.
- As séries coletadas não incluem `amm_op_latency_seconds_bucket`.

## Integrações opcionais

Quando presentes na pasta `scripts/`, os artefatos abaixo são executados em sequência após a coleta e possuem exit codes propagados:

1. `obs3_quality_checks.py`
2. `obs3_hash_manifest.py`
3. `obs3_verify_manifest.py`

Ausências são ignoradas silenciosamente (com logging). Falhas interrompem o runner.

## Boas práticas

- Execute o runner somente em ambientes confiáveis; nunca exponha `:9090` para redes públicas.
- Faça rotação periódica do diretório `prom-data` usado em desenvolvimento para evitar crescimento indefinido.
- Versione os artefatos produzidos (logs + evidências) junto com o commit de observabilidade.
- Ajuste `--ready-attempts`/`--ready-sleep` conforme SLAs locais para evitar falsos negativos.
- Combine com watchers/gates das demais threads para garantir cobertura completa do pack OBS-3.
