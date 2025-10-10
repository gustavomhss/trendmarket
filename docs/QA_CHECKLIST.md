# OBS-3 — QA Checklist (RAG)

## Legenda RAG

- **G (Green):** requisito atendido sem pendências.
- **A (Amber):** parcialmente atendido ou aguardando evidência adicional.
- **R (Red):** falha ou bloqueio crítico; ação imediata necessária.

Atualize o campo **Status** após executar cada verificação e anexe evidências em `out/obs_gatecheck/evidence/` ou repositório equivalente.

## 1. Scrapes (DEV/PROD)

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| `prometheus.dev.yml` válido | `promtool check config ops/prometheus/prometheus.dev.yml` | `Checking ops/prometheus/prometheus.dev.yml
  SUCCESS: 0 rule files found` | `out/obs_gatecheck/evidence/promtool_dev_check.log` | A |
| `prometheus.prod.yml` válido | `promtool check config ops/prometheus/prometheus.prod.yml` | `Checking ops/prometheus/prometheus.prod.yml
  SUCCESS: 0 rule files found` | `out/obs_gatecheck/evidence/promtool_prod_check.log` | A |

## 2. Recording Rules

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Sintaxe das rules | `promtool check rules ops/prometheus/rules/core.rules.yml` | `Checking rules file ops/prometheus/rules/core.rules.yml
  SUCCESS` | `out/obs_gatecheck/evidence/promtool_rules_check.log` | A |

## 3. Testes de Rules

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Suite unitária | `promtool test rules ops/prometheus/tests/core.rules.test.yml` | `Unit Testing: ops/prometheus/tests/core.rules.test.yml
  PASS` | `out/obs_gatecheck/evidence/promtool_rules_test.log` | A |

## 4. Runner (Thread 4)

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Coleta DEV | `./scripts/obs_t3_prom_scrape_run.sh --config ops/prometheus/prometheus.dev.yml --out out/obs_gatecheck` | Exit code 0; JSONs `prom_up_dev.json`, `prom_latency_dev.json`, manifesto parcial. | `out/obs_gatecheck/evidence/prom_up_dev.json` | A |
| Coleta PROD | `./scripts/obs_t3_prom_scrape_run.sh --config ops/prometheus/prometheus.prod.yml --out out/obs_gatecheck` | Exit code 0; JSONs `prom_up_prod.json`, `prom_latency_prod.json`, readiness true. | `out/obs_gatecheck/evidence/prom_up_prod.json` | A |

## 5. Quality Checks (Thread 5)

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Checks completos | `python3 scripts/obs3_quality_checks.py --evidence-dir out/obs_gatecheck/evidence --strict` | Manifesto `prom_scrape.json` com `quality_checks.*: true` e `p95_le_4x_avg: true`. | `out/obs_gatecheck/evidence/prom_scrape.json` | A |

## 6. Manifest Hash & Metadata (Thread 6)

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Manifesto assinado | `python3 scripts/obs3_hash_manifest.py --evidence-dir out/obs_gatecheck/evidence --verbose` | Log com `integrity map written` e manifesto atualizado com `sha256`. | `out/obs_gatecheck/evidence/prom_scrape.json` | A |

## 7. Schema Verify (Thread 7)

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Manifesto conforme schema | `python3 scripts/obs3_verify_manifest.py --manifest out/obs_gatecheck/evidence/prom_scrape.json --schema ops/schemas/manifest.schema.json` | `validation succeeded` | `out/obs_gatecheck/evidence/prom_scrape_schema.log` | A |

## 8. CI (Thread 8)

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Workflow configurado | Revisar [`docs/CI_OBS3.md`](CI_OBS3.md) e `.github/workflows/obs3-prometheus-ci.yml` | Jobs `lint`, `promtool`, `evidence`, `upload-artifact` com `permissions: actions: write`. | `docs/CI_OBS3.md` | A |

## 9. Anti-Scans

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Scans sem findings | `make pr-check` ou `scripts/obs_policy_scan.sh --evidence out/obs_gatecheck/evidence` | Saída sem `BLOCKER`/`placeholder`. | `out/obs_gatecheck/evidence/obs_policy_scan.log` | A |

## 10. Cardinalidade

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Limite `amm_*` | `promql> count by (__name__) (amm_op_latency_seconds_bucket)` (Prometheus console) | ≤ 300 séries para `amm_*` em dev; snapshot no manifesto. | `out/obs_gatecheck/evidence/cardinality_snapshot.json` | A |
| Telemetria Prometheus | `promql> sum(prometheus_tsdb_head_series)` | Valor estável (<100k) em dev. | `out/obs_gatecheck/evidence/prom_tsdb_series.json` | A |

## 11. Segurança

| Item | Comando | Saída esperada | Evidência (path/link) | Status (R/A/G) |
| --- | --- | --- | --- | --- |
| Porta interna | `ss -lnt | grep :9090` | Bind em `127.0.0.1:9090` ou interface privada. | `out/obs_gatecheck/evidence/prometheus_bind.txt` | A |
| Sanitização de métricas | Revisão dos JSONs `prom_*` | Sem PII/segredos; labels apenas `op,service,env,version`. | `out/obs_gatecheck/evidence/prom_sanitized_review.md` | A |

---

**Assinaturas**

- Revisor(a): ______________________
- Data: ____/____/______
- Observações: ______________________________________________________________
