# OBS-3 — Executive Summary

**Objetivo.** Garantir SLIs estáveis e auditáveis para as operações críticas do TrendMarket por meio de scrapes Prometheus versionados, recording rules testadas e um pipeline de evidências com manifesto criptograficamente íntegro.

**Escopo.** Inclui configurações de scrape para dev/prod (`ops/prometheus/prometheus.{dev,prod}.yml`), discovery dinâmico com `file_sd`, recording rules (`ops/prometheus/rules/core.rules.yml`), suíte de testes (`ops/prometheus/tests/core.rules.test.yml`), runner de evidências (`scripts/obs_t3_prom_scrape_run.sh`), verificações de qualidade (`scripts/obs3_quality_checks.py`), manifesto/hash/schema (`scripts/obs3_hash_manifest.py`, `scripts/obs3_verify_manifest.py`, `ops/schemas/manifest.schema.json`) e automações de CI descritas em [`docs/CI_OBS3.md`](CI_OBS3.md). Exclui dashboards remotos, alertas externos e deploy automatizado.

**Premissas.** Labels estáveis (`op`, `service`, `env`, `version`), histogramas `amm_op_latency_seconds_*` com buckets cobrindo a cauda (até ≥ 5 s), janelas de avaliação de 5 minutos com scrapes a cada 30 s, endpoints `/metrics` respondendo em <1 s, e tokens de GitHub Actions com permissões `actions:write`/`contents:read`.

**Arquitetura.**

```
App (:9464 /metrics) ─┐
                      ├─ Prometheus (local :9090) → Rules → Evidências (JSON)
OTel Collector (:8888)┘
```

Prometheus local mantém paridade entre ambientes; PROD usa `file_sd` para múltiplos clusters e aplica relabel para hygiene de labels. Recording rules consolidam métricas `avg`, `p75`, `p95` e counters operacionais; evidências são materializadas em `out/obs_gatecheck/evidence/` e assinadas via manifesto.

**Entregáveis.** Configurações de scrape, rules e testes; scripts de runner, qualidade, hashing e verificação; schemas; documentação operacional; workflow de CI; troubleshooting e checklists RAG.

**Riscos & Mitigações.** Cardinalidade descontrolada (mitigada por labels fixas e `cardinality_snapshot`), quantis instáveis (quality checks rejeitam `p95 > 4× avg`), targets indisponíveis (`obs_t3_prom_scrape_run.sh` falha rápido e reporta readiness), crescimento do TSDB (retention ajustável em `prometheus.prod.yml`, monitoramento com métricas `prometheus_tsdb_*`).

**Critérios de Aceite.** `promtool check config`/`promtool check rules`/`promtool test rules` sem erros, evidências contendo `p75/p95` numéricos e manifesto válido pelo schema, quality checks com todos os flags `true`, CI verde para jobs OBS-3, ausência de placeholders ou strings bloqueadas.

**Como validar.** (1) `make lint` ou `yamllint ops/prometheus` + `ruff scripts/obs3_*.py`. (2) Executar `scripts/obs_t3_prom_scrape_run.sh --env dev` seguido de `python3 scripts/obs3_quality_checks.py --strict`. (3) Finalizar com `python3 scripts/obs3_verify_manifest.py --manifest out/obs_gatecheck/evidence/prom_scrape.json`.

**Governança.** CODEOWNERS exigindo dupla revisão para `ops/prometheus/` e `scripts/obs3_*`, workflows em [`docs/CI_OBS3.md`](CI_OBS3.md) com permissões restritivas e anti-scans ativos (`scripts/obs_policy_scan.sh`).

**Contato / Ownership.** Time de observabilidade (responsável por scrapes, rules e evidências) e time de QA (responsável por checklist e governança de CI).
