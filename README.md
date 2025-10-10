# OBS-3 — Prometheus Scrape & Validação de Métricas (CRD-8)

[![CI](https://github.com/gustavomhss/trendmarket/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/gustavomhss/trendmarket/actions/workflows/ci.yml)
[![Docs Guard (Agents)](https://github.com/gustavomhss/trendmarket/actions/workflows/docs-guard-agents.yml/badge.svg?branch=main)](https://github.com/gustavomhss/trendmarket/actions/workflows/docs-guard-agents.yml)

## TL;DR

- Scrapes dedicados para dev (`prometheus.dev.yml`) e prod (`prometheus.prod.yml`) com `file_sd` e hygiene de labels.
- Recording rules para `avg`, `p75` e `p95` com testes automatizados e buckets alinhados ao SLO.
- Runner de evidências (`obs_t3_prom_scrape_run.sh`) gera JSONs assinados, valida readiness e falha rápido sem dados.
- Quality gate automatizado (`obs3_quality_checks.py`) cruza métricas com limites auditáveis e produz relatórios em `out/obs_gatecheck/evidence/`.
- Manifesto versionado com hashing (`obs3_hash_manifest.py`), schema enforcement (`obs3_verify_manifest.py`) e jobs de CI (`docs/CI_OBS3.md`).

## Visão Geral

O objetivo do OBS-3 é garantir que os SLIs expostos por serviços monitorados sejam coletados por Prometheus com rastreabilidade completa e evidências auditáveis. Os fluxos asseguram paridade entre ambientes, validação de regras e um gate automatizado que impede regressões de observabilidade.

**Escopo incluso (Threads 1–9):** configurações de scrape Prometheus, recording rules para métricas de latência e throughput, testes de regras, runner de coleta e qualidade, manifesto de evidências (hash + schema), integrações de CI, documentação operacional e roteiros de troubleshooting.

**Escopo excluído:** criação de dashboards remotos (Grafana, Looker), alertas externos ao Prometheus local, automações de deploy e ingestão em sistemas third-party.

**Entregáveis principais:**

- Configurações de scrape: [`ops/prometheus/prometheus.dev.yml`](ops/prometheus/prometheus.dev.yml) e [`ops/prometheus/prometheus.prod.yml`](ops/prometheus/prometheus.prod.yml).
- Targets e discover: [`ops/prometheus/targets-prod.json`](ops/prometheus/targets-prod.json) e [`ops/prometheus/targets-otelcol-prod.json`](ops/prometheus/targets-otelcol-prod.json).
- Recording rules e testes: [`ops/prometheus/rules/core.rules.yml`](ops/prometheus/rules/core.rules.yml) e [`ops/prometheus/tests/core.rules.test.yml`](ops/prometheus/tests/core.rules.test.yml).
- Runner e qualidade: [`scripts/obs_t3_prom_scrape_run.sh`](scripts/obs_t3_prom_scrape_run.sh), [`scripts/obs3_quality_checks.py`](scripts/obs3_quality_checks.py), [`scripts/obs3_hash_manifest.py`](scripts/obs3_hash_manifest.py) e [`scripts/obs3_verify_manifest.py`](scripts/obs3_verify_manifest.py).
- Schemas e evidências: [`ops/schemas/manifest.schema.json`](ops/schemas/manifest.schema.json) e artefatos em `out/obs_gatecheck/evidence/`.
- Automação de CI: [`docs/CI_OBS3.md`](docs/CI_OBS3.md) descreve jobs, permissões e políticas de publicação.

## Arquitetura

```
App (:9464 /metrics) ─┐
                      ├─ Prometheus (local :9090) → Rules → Evidências (JSON)
OTel Collector (:8888)┘
```

- Paridade entre DEV ↔ PROD garantindo janelas e limites idênticos.
- `file_sd_configs` em PROD para apontar múltiplos clusters sem rebuild.
- Labels normalizados (`env`, `service`, `op`, `version`) evitam explosão de cardinalidade.
- Buckets e quantis configurados para latência com cortes `p50/p75/p95/p99` e histogramas coerentes com janelas de 30s e avaliação a cada 5m.

## Quickstart (Local)

### Pré-requisitos

Certifique-se de ter `prometheus`, `promtool`, `python3`, `jq`, `yamllint`, `shellcheck` e `ruff` disponíveis no PATH.

### Comandos essenciais

```bash
make lint
make run                   # inicia Prometheus local e coleta evidências básicas
make evidence              # runner + quality (T5) + hash (T6) + schema verify (T7)
make pr-check              # verificador único local (T9)
```

Os relatórios e manifestos são gravados em `out/obs_gatecheck/evidence/`. As execuções de `make run` expõem Prometheus em `http://localhost:9090` com targets definidos em `ops/prometheus/prometheus.dev.yml`.

## Critérios de Aceite

- `promtool check rules ops/prometheus/rules/core.rules.yml` e `promtool test rules ops/prometheus/tests/core.rules.test.yml` sem erros ou warnings.
- Evidências finais contendo métricas `p75` e `p95` numéricas, além de manifesto com hash SHA-256 válido conforme o schema JSON.
- Runner e quality gate falham rapidamente quando não há métricas disponíveis, evitando falsos positivos.
- Ausência de placeholders, conflitos de merge ou strings bloqueadas; pipeline descrito em [`docs/CI_OBS3.md`](docs/CI_OBS3.md) executando com sucesso (Thread 8).

## Troubleshooting

| Sintoma | Causa provável | Ação recomendada |
| --- | --- | --- |
| Porta `:9090` indisponível | Outro Prometheus/serviço em execução | Finalize o processo existente ou ajuste `--web.listen-address` em `prometheus.dev.yml`. |
| Target aparece com `up == 0` | Endpoint `/metrics` inacessível ou scrape interval incorreto | Confirme reachability, valide TLS e ajuste `scrape_interval` conforme ambiente. |
| Latências `p75/p95` vazias | Buckets insuficientes ou service sem tráfego | Ajuste `histogram buckets` em `core.rules.yml` e valide geração com tráfego sintético. |
| `promtool test rules` falha com "insufficient data" | Janelas menores que os testes exigidos | Amplie a janela no arquivo de teste ou execute scrapes adicionais antes de rodar o teste. |
| `yamllint` ou `shellcheck` não encontrados | Tooling ausente no PATH | Instale via gerenciador local (`pip install yamllint`, `apt-get install shellcheck`). |
| `actions/upload-artifact` retorna 403 | Token de GitHub Actions sem permissões corretas | Atualize `permissions` no workflow `docs/CI_OBS3.md` ou use PAT com `actions: write`. |
| Anti-scan bloqueia string sensível | Dados ou secrets vazaram para configs/testes | Remova o conteúdo, regenere evidências sem segredos e revalide com `make pr-check`. |

## Segurança

- Mantenha o serviço do Prometheus (`:9090`) restrito à rede interna ou com túnel seguro; não exponha endpoints sem autenticação.
- Métricas não devem conter PII ou segredos; normalize labels e valores antes da exportação.
- Revisão dupla via CODEOWNERS é obrigatória para alterações em `ops/prometheus/` e `scripts/obs3_*`, garantindo rastreabilidade.

## Referências Cruzadas

- Guia detalhado de scrapes e rules: [`ops/prometheus/README.md`](ops/prometheus/README.md).
- Documentação dos scripts e automações: [`scripts/README.md`](scripts/README.md).
- Sumário executivo: [`docs/EXEC_SUMMARY.md`](docs/EXEC_SUMMARY.md).
- Checklist de QA: [`docs/QA_CHECKLIST.md`](docs/QA_CHECKLIST.md).
- Especificação de CI: [`docs/CI_OBS3.md`](docs/CI_OBS3.md).
