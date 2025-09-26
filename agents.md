# AGENTS.md — Guia definitivo para trabalhar neste repositório (Codex Edition)

> **Status:** Padrão Ouro • Sem placeholders • Com governança, SLOs, hooks A110 e watchers em toda parte.
> **Norte (NSM):** *Time‑to‑Preço‑Válido* p75 ≤ **0,5 s** (p95 ≤ **0,8 s**).
> **Doutrina:** Excellence‑First (melhor de primeira), com rastreabilidade Requisitos↔Packs↔Watchers↔Hooks.

---

## 0) Sumário executivo
O **CreditEngine$ (CE)** é o sistema nervoso de **decisão e precificação de crédito**. Ele integra **dados com CDC e contratos de dados**, **mecanismos de preço/mercado** (leilões, FX, oráculos), **serving & monitoramento de modelos**, **observabilidade full‑stack (OTel)** e **gates automáticos (A110)** que acoplam **métricas→ações** (rollback, degrade, pause, failover) com **owners** claros.

**Resultados esperados:**
- Decisões auditáveis em < 0,8 s p95, com trilha completa e contrato de dados válido.
- Resiliência operacional (BCDR testado), experimento A/B com SRM OK, privacidade por padrão.
- Reprodutibilidade: containers + lockfiles + artefatos versionados (sha256/commit/trace/audit).

---

## 1) Arquitetura do sistema (alto nível)
**Domínios principais**
1. **DEC (Decision & Pricing):** Core de decisão; orquestra hooks A110; limites de latência; rollback/degrade.
2. **PM (Mercados & Sinais):** Leilões reversos (originação), roteamento FX, oráculos (TWAP/failover), regras anti‑arb.
3. **ML (Model Lifecycle):** Serving (Triton/ONNX), monitoramento (PSI/KS), experimentos A/B com SRM.
4. **DATA (Dados):** CDC → Lakehouse (Iceberg), dbt, contratos A106/A87/A89, schema registry e expectativas.
5. **PLAT/SRE:** Observabilidade (OTel), SLO/SLI, gestão de erros e capacidade, circuit‑breakers.
6. **FE/SDK:** Portais internos, APIs e SDKs, CWV verdes (INP p75 ≤ 200 ms), versionamento de contrato.
7. **SEC/PRIV:** Segurança de supply chain, chaves/rotação, privacidade (A108), DP/consentimentos.

**Invariantes e princípios**
- **No‑arb** em oráculos/FX/derivativos; **paridade** call‑put e **convexidade** em superfícies.
- **Idempotência** de eventos; **logs imutáveis**; **compatibilidade retroativa** em contratos.
- **Observabilidade default‑on**; hooks A110 com janela/limiar/ação/owner/rollback definidos.

---

## 2) Layout de diretórios (padrão recomendado)
```
/ops/
  evidence/           # Evidence JSON por pack e agregado
  tests/
    probes/           # Probes (≥20 por pack)
    qgen/             # Q/A gerado (≥20 por pack)
  watchers/           # Regras/params dos watchers
  scripts/            # Runner, dry‑run, gatilhos sintéticos
/web/                 # FE (Next.js) + pnpm-lock.yaml
/be/                  # FastAPI (Python 3.11) + uv.lock
/data/                # CDC, contratos, schemas, dbt
/ml/                  # Export ONNX, Triton configs, monitores (PSI/KS)
/specs/               # ADRs, PR‑FAQ, hooks A110 (YAML), ACE/DoR/DoD
/infra/               # IaC, pipelines, alerts, dashboards
/docs/                # Este AGENTS.md + READMEs por domínio
```
**Diretrizes:** cada diretório deve expor **Make/CLI** com alvos de *lint/test/build/run/evidence/hooks.dry*; arquivos de lock e `container` declarados.

---

## 3) Convenções de código e padrões por domínio
### 3.1 Backend (BE/API)
- **Stack:** Python 3.11 + FastAPI, tipagem estrita, Pydantic para contratos.
- **SLI/SLO:** p95 ≤ 800 ms; 5xx zero‑tolerance em release.
- **Observabilidade:** OTel (traces `decision.core`, `auction.match`, `fx.router`), logs estruturados com `trace_id`, `pack_id`, `hook_id`.
- **Testes:** unit + property‑based (invariantes), integração, contratos (compat/backward), golden tests (endereços/IDs fixos).

### 3.2 Dados (CDC/Lakehouse/dbt)
- Debezium → Iceberg; dbt com *tests* (`unique`, `not_null`, *expectations*).
- **SLA:** lag p95 ≤ 120 s; offsets monotônicos; schema registry sem incompatibilidades.
- **Contratos:** A106/A87/A89 — owners, SLAs de *freshness/completeness/consistency*, versionamento.

### 3.3 ML (Serving/Monitoring/Experimentos)
- Export ONNX; servir via Triton/TorchServe (fallback); telemetria de latência/inferência.
- **Drift:** PSI ≤ 0,2; KS ≤ 0,1; hooks de rollback do baseline.
- **A/B:** SRM obrigatório; AA tests periódicos; guardrails e *no peeking*.

### 3.4 FE/SDK
- TypeScript + Next.js; padrões de acessibilidade; CWV (INP p75 ≤ 200 ms); *feature flags* e *canary*.

### 3.5 Streaming
- Kafka/Redpanda + Flink/Spark SS; partitions balanceadas; SLAs e2e alinhados ao DEC.

### 3.6 Segurança/Privacidade
- RBAC, rotação de chaves, CSP/CSRF/Trusted Types; PII fora de ambientes não‑prod; DPIA/PIA vinculada a ADRs.

---

## 4) Watchers obrigatórios e Hooks A110 (defaults)
**Watchers mínimos em todo o repo**: `api_breaking_change_watch`, `schema_registry_drift_watch`, `data_contract_break_watch`, `dbt_test_failure_watch`, `cdc_lag_watch`, `slo_budget_breach_watch`, `model_drift_watch`, `metrics_decision_hook_gap_watch`, `formal_verification_gate_watch`, `web_cwv_regression_watch`, `okr_risk_alignment_watch`, `dp_budget_breach_watch`, `runtime_eol_watch`, `dep_vuln_watch`, `oracle_divergence_watch`, `fx_delta_benchmark_watch`.

**Defaults por domínio (pode sobrescrever):**
- **DEC:** `metrics_decision_hook_gap_watch`, `model_drift_watch`, `slo_budget_breach_watch` → Hook: `latency.p95>800•5m->degrade_route; owner=SRE; rollback=yes`.
- **PM:** `oracle_divergence_watch`, `fx_delta_benchmark_watch`, `auction_invariant_breach_watch`, `slo_budget_breach_watch` → Hook: `staleness_ms>30000•5m->switch_to_twap_failover; owner=BC; rollback=yes`.
- **ML:** `model_drift_watch`, `ab_srm_watch`, `runtime_eol_watch`, `image_vuln_regression_watch` → Hook: `psi>0.2•24h->rollback_model; owner=ML; rollback=yes`.
- **DATA:** `cdc_lag_watch`, `schema_registry_drift_watch`, `dbt_test_failure_watch`, `doc_coverage_watch` → Hook: `contract_break->rollback+waiver_timebox; owner=DATA; rollback=yes`.
- **PLAT:** `tracing_sampling_watch`, `alert_storm_watch`, `slo_budget_breach_watch`, `policy_violation_watch` → Hook: `sample_rate<1%•15m->block_release; owner=SRE; rollback=yes`.
- **FE:** `web_cwv_regression_watch`, `api_breaking_change_watch` → Hook: `inp.p75>200ms•24h->rollback_FE; owner=FE; rollback=yes`.
- **SEC/PRIV:** `dep_vuln_watch`, `image_vuln_regression_watch`, `idp_keys_rotation_watch`, `dp_budget_breach_watch`, `formal_verification_gate_watch` → Hook: `privacy_budget>1.5x•1h->freeze; owner=SEC; rollback=yes`.
- **INT:** `api_breaking_change_watch`, `cache_ttl_misuse_watch`, `cls_payin_cutoff_watch` → Hook: `contract_tests_fail_pct>0->block_release; owner=INT; rollback=yes`.

**Biblioteca A110 (exemplos de gramática)**
```yaml
hook: fx-oracle-staleness
kpi: staleness
threshold: 30s
window: 5m
action: switch_to_twap_failover
owner: BC
evidence: trace:oracle.fetch;audit:oracles/2025w36
rollback: yes
```

---

## 5) Métricas, SLOs e dicionário (com cálculo/ação)
- **Time‑to‑Preço‑Válido:** latência DEC p75/p95; **Hook:** `latency.p95>800ms•5m -> degrade_route`.
- **CDC lag p95:** alvo ≤ 120 s; **Hook:** `lag>120s•5m -> degrade_to_hot_table + tickets`.
- **Drift (PSI/KS):** PSI ≤ 0,2; KS ≤ 0,1; **Hook:** `psi>0.2•24h -> rollback_model`.
- **Staleness de oráculo:** < 30 s; **Hook:** `>30s•5m -> TWAP+failover`.
- **SRM A/B:** SRM OK; **Hook:** `srm_fail•run -> pause+audit`.
- **CWV INP p75:** ≤ 200 ms; **Hook:** `inp>200ms•24h -> rollback_FE`.

Cada KPI deve definir **fonte**, **janela**, **cálculo**, **hook**, **owner**, **painel** e **evidência**.

---

## 6) Dados & contratos (A106/A87/A89)
- **Contrato mínimo:** owner, schema version, semântica de campos, SLOs (freshness/completeness/consistency), políticas de retenção/privacidade.
- **Política de schema:** compatibilidade retroativa (evolução *additive*), DRL para mudanças rompedoras (waiver + plano de migração).
- **CDC:** replication slots/heartbeats; DLQ; compactions; *degrade* para tabela quente sob pressão.

**Checklists**
- [ ] Data contract versionado e publicado
- [ ] dbt tests verdes (`unique`, `not_null`, `expectations`)
- [ ] Schema registry sem incompatibilidades
- [ ] CDC lag ≤ 120 s p95 (alertas linkados a hooks)

---

## 7) Mercado: Leilões, FX e Oráculos
### 7.1 Leilão reverso (originação)
- Ordenar por custo/risco efetivo; acumular até K*; empate por prioridade determinística (tempo/seed pública); monotonicidade e *budget‑balance*; penalidades anti‑desvio.

### 7.2 FX e roteamento
- VPIN como filtro de *toxicity*; spreads adaptativos; *circuit‑breakers*; benchmarking de deltas; fallback TWAP 5m sob staleness ou desvio.

### 7.3 Oráculos
- **Invariantes:** heartbeats; staleness<30s; `TWAP` como fallback; assinaturas e audit trail.

---

## 8) Observabilidade (OTel) e painéis
- **Traces (exemplos):** `decision.core`, `auction.match`, `fx.router`, `oracle.fetch`, `cdc.reader`, `dbt.run`, `ml.infer`.
- **Atributos:** `trace_id`, `pack_id`, `hook_id`, `latency_ms`, `status`, `audit_id`.
- **Eventos:** `hook.trigger`, `rollback.apply`, `slo.violation`.
- **Dashboards mínimos:** Latency/Error; Hook Coverage; CDC Lag; Drift (PSI/KS); CWV; SLO burn rate.

---

## 9) Segurança & Privacidade (A77/A108)
- **Princípios:** minimização, *need‑to‑know*, mascaramento em não‑prod, retenção limitada, PII fora de logs, RBAC forte, CSP/CSRF/Trusted Types.
- **Processo:** DPIA/PIA anexada a ADRs; rotação de segredos (≤90d); dependências e imagens sem vulnerabilidades altas.

---

## 10) Testes, Evidence e Golden Notebooks
- **Por pack:** *Probes* (≥20), *QGen* (≥20), *Hard‑negatives* (≥10), Evidence JSON com KPIs ≠ null quando aplicável.
- **Golden Notebooks:** para domínios com matemática/mercado (CIP/AMM/vol/superfície), com invariantes e PNGs exportados.
- **Relatórios automáticos:** Lint Matemático (paridade/convexidade/calendar/CIP/no‑crossing), Watchers, Evidence agregado.

---

## 11) CI/CD, gates e fluxos Git
- **Branches:** `main` protegida; `feature/*` com CI completo; *canary* obrigatório em rollouts sensíveis.
- **Gate de promoção:** watchers **verdes**, hooks ativos, Evidence publicado, ADR/waivers versionados.
- **Tripla revisão:** Conteúdo • Técnica/CI • Conformidade (frente v3.1 + Simon v2.8).
- **Checklists essenciais:**
  - [ ] No‑Go passado (história com ACE/DoR/DoD + watcher + owner + hook)
  - [ ] Contracts OK (A106/A87/A89)
  - [ ] Telemetria/OTel presente
  - [ ] PR‑FAQ/README com limites e SLO

---

## 12) Tabela de escolhas por domínio (padrão & alternativas)
| Domínio | Padrão | Alternativa (quando) | SLO/SLA | Watchers chave | Acão A110 |
|---|---|---|---|---|---|
| **BE/API** | Python 3.11 + FastAPI | Go (I/O extremo) | p95 ≤ 800 ms | 5xx/SLO | rollback_release |
| **Dados** | Debezium + Iceberg + dbt | Delta/Hudi | lag p95 ≤ 120 s | cdc/schema/dbt | degrade_to_hot_table |
| **ML** | Triton/ONNX | TorchServe | PSI ≤ 0,2 | drift | rollback_baseline |
| **FE** | TS + Next.js | RN/Flutter | INP ≤ 200 ms | CWV | rollback_feature |
| **Stream** | Kafka/Flink | — | e2e ≤ alvo | SLO | throttle/retry |

---

## 13) Incident & BCDR (runbooks)
- **Tabletop** trimestral; DR drill semestral; RPO≤5 min; RTO≤30 min.
- **Cenários comuns:** CDC lag>120s, SRM fail, INP>200ms, region down, FX shock, schema drift, dbt fail.
- **Padrão:** acionar hook → evidenciar `trace_id`/`audit_id` → comunicar → rollback/degrade/pause → *postmortem* com ações datadas (overdue=0).

---

## 14) Como configurar o ambiente
- **Containers** por domínio; **locks** (`uv.lock`, `pnpm-lock.yaml`); seeds determinísticas.
- **CLI/Make**: `env.up`, `be.test`, `data.run`, `ml.serve`, `hooks.dry`, `watchers.dry`, `evidence.publish`.
- **Artefatos**: versionar `sha256`, `commit`, `trace_id`, `audit_id` em cada release.

---

## 15) Fluxo de trabalho Git
1. Abrir PR com ADR/PR‑FAQ, ACE/DoR/DoD, packs e hooks.
2. CI: contratos + unit + property + lint + watchers.
3. Canary com *feature flag*; monitorar SLO/SLIs e Hook Coverage.
4. Promover com Evidence + changelog + statusboards.

---

## 16) Papéis, owners e RACI
- **PO** (orquestra, gates, waivers), **ST** (experimental/AB), **PY/DC** (dados/CDC/dbt), **ML**, **SRE/PLAT**, **FE**, **SEC/RISK**. Cada história deve ter **owner** e **hook** claros.

---

## 17) Top 12 riscos → Watchers/Hooks
- Drift, CDC lag, staleness, SRM, CWV, API 5xx, Privacidade, Contrato, Divergência de oráculo, Runtime EOL, Dependências, FX benchmark.
- Para cada risco: **indicador → watcher → hook → mitigação → evidência** documentados.

---

## 18) Políticas de No‑Go
- História sem ACE/hook/watchers → **No‑Go**.
- Watcher crítico sem owner → **No‑Go**.
- Métrica crítica sem ação A110 → **No‑Go**.
- Falta de evidência executável → **No‑Go**.
- Violação de privacidade (A108) → **No‑Go** + escalonamento.

---

## 19) Atualização deste arquivo
- Atualizar **sempre** que houver mudança de processo, adição de watchers/hooks, alteração de SLOs/KPIs, ou novas dependências.
- **Responsável:** PO do repositório (delegável), com revisão técnica de SRE/Dados/ML/FE.

---

## 20) Anexos utilitários
### 20.1 Templates canônicos
**Backlog (CSV)**
```
epic_id,story_id,title,ace_given,ace_when,ace_then,dor,dod,packs,watchers,hook_id,owner,estimate,priority,evidence_links,status
```
**Sprint (YAML)**
```yaml
sprint: <N>
start: <utc_iso>
end: <utc_iso>
capacity: {PO:0.5, ST:1.0, PY:1.0, DC:0.5, ML:1.0, SRE:0.5, FE:0.5}
stories:
  - id: S-001
    title: ...
    ace: {given: ..., when: ..., then: ...}
    hook: <id>
    owner: <papel>
    estimate: 3
    evidence: ["..."]
```
**Hook A110 (YAML)**
```yaml
hook: <id>
kpi: <KPI>
threshold: <valor>
window: <janela>
action: <ação>
owner: <papel>
evidence: <artefato>
rollback: yes
```
**ADR (Markdown)**
```
ID • Contexto • Requisito/SLO • Opção‑Padrão • Alternativas • Trade‑offs • Decisão • Packs • Watchers • Hook A110 • Rollback • Owners • ACE/DoR/DoD • Evidências
```

### 20.2 Glossário mínimo
- **A110:** Tabela de *decision hooks* (métrica→ação).
- **ACE/DoR/DoD:** Given/When/Then + critérios de entrada/saída.
- **SRM:** *Sample Ratio Mismatch* (A/B).
- **PSI/KS:** métricas de *drift*.
- **TWAP:** média ponderada no tempo, fallback de oráculo.
- **CIP:** paridade de juros coberta (FX).
- **CWV/INP:** Web Vitals / Interaction to Next Paint.

---

### 20.3 Checklist de release (Gold)
- [ ] Watchers verdes
- [ ] Hooks A110 ativos (coverage=100%)
- [ ] Evidence JSON por pack + agregado
- [ ] Readmes/PR‑FAQ atualizados
- [ ] Changelog + statusboards publicados

> **Fim.** Este arquivo é a *spine* operacional para codar, revisar, operar e auditar o CreditEngine$. Se algo aqui conflitar com um contrato ou política vigente, prevalece o contrato/política com ADR e hook correspondente.



---

# Add‑On Pack: *OSCAR EDITION* — Mais profundo, mais técnico, mais operacional

> Este apêndice estende o AGENTS.md com especificações **prontas‑para‑uso**: contratos de API, esquemas de dados, YAMLs de CI/CD, invariantes matemáticos (mercado/FX), fórmulas de drift, pipelines de observabilidade, runbooks, perf budgets, SDKs de referência e *golden artifacts*. Tudo está alinhado às políticas, watchers e hooks A110 definidos acima.

## A) Mapa de execução e limites de latência (budget por camada)
```
[ Edge/FE ] 40–80ms  →  [ API Gateway ] 20–50ms  →  [ DEC Core ] 180–300ms  →  [ Mercado/Oráculos ] 80–150ms  →  [ CDC/Data ] 60–120ms  →  [ Persist/Audit ] 20–40ms
                                   ↘ [ ML Inference ] 40–120ms
```
**Orçamento p95:** 800ms. **Reserva:** 15% para variação ambiente.

## B) Contratos de API (canon)
### B.1 `POST /api/v1/decision/price`
**Idempotência:** `Idempotency-Key` obrigatório.
**Tracing:** `traceparent`, `x-pack-id`, `x-hook-id` propagados.

**Request (JSON)**
```json
{
  "app_id": "a-123",
  "ts": "2025-09-23T12:34:56Z",
  "applicant": {
    "document": "***masked***",
    "age": 31,
    "region": "BR-BA"
  },
  "product": {"code": "PX-001", "amount": 5000, "tenor_months": 12},
  "context": {"channel": "web", "fx_pair": "USD/BRL"}
}
```
**Response (JSON)**
```json
{
  "decision": "APPROVED",
  "price": {"apr": 0.298, "monthly": 487.23, "fx_rate": 5.11},
  "explanations": ["risk_segment=B2", "model=v15.2.1"],
  "audit": {"trace_id": "...", "pack_id": "px-core-v15", "model_sha": "..."}
}
```
**Erros padrão**
- `400` contrato inválido (detalha campo/semântica)
- `409` idempotência em conflito
- `422` dados insuficientes (violação A106/A87)
- `429` budget de SLO queimando → *degrade route*
- `503` dependência crítica indisponível (com hook aplicado)

### B.2 `GET /api/v1/audit/:trace_id`
- Retorna *timeline* de hooks, watchers disparados, e *snapshot* dos dados/parametrizações utilizadas (hashadas e referenciadas).

## C) Esquemas e contratos de dados (A106/A87/A89) — exemplos prontos
### C.1 Evento `loan.application.created` (CDC/stream)
```json
{
  "$schema": "https://schemas.creditengine/loan.application.created-1.3.0.json",
  "owner": "DATA",
  "version": "1.3.0",
  "privacy": {"level": "PII", "masking": "tokenized", "retention_days": 365},
  "fields": {
    "app_id": {"type": "string", "unique": true},
    "ts": {"type": "timestamp", "freshness_slo_s": 120},
    "region": {"type": "string", "enum": ["BR-BA", "BR-SP", "BR-RJ", "LATAM"]},
    "amount": {"type": "number", "min": 100, "max": 100000},
    "channel": {"type": "string"}
  },
  "dbt_tests": ["unique:app_id", "not_null:ts", "expectation:freshness<=120s"],
  "compat": {"policy": "backward-additive"}
}
```
### C.2 Tópicos CDC
- `cdc.loan_applications.v1` → Iceberg: `lake/bronze/loan_apps` (compaction diário; z-order por `ts`)
- `cdc.offer_decisions.v1` → Iceberg: `lake/silver/offer_decisions`

## D) Mercado (FX/Oráculos/Leilões) — invariantes e rotinas
### D.1 CIP — *Covered Interest Parity*
```
F = S * (1 + r_d * T) / (1 + r_f * T)
```
- **No‑arb:** `|F_mkt − F_CIP| ≤ ε` (ε calibrado). Hook: `oracle_divergence_watch`.
- **TWAP(Δ):** `TWAP_t = Σ_{i=t-Δ}^{t} w_i * S_i` com `Σw_i=1`. Usar Δ=5m em fallback.

### D.2 Staleness & Failover
- **Staleness < 30s** p95. Hook: `switch_to_twap_failover` + **owner** BC + rollback.

### D.3 Leilão reverso (originação)
- Monotonicidade de *score→preço*, tie‑break determinístico (`ts`, `seed pública`).
- *Budget‑balance* e penalidades anti‑desvio (outlier rejection + *cooldown*).

## E) Drift & Monitoramento de Modelos
### E.1 PSI (Population Stability Index)
```
PSI = Σ (p_i − q_i) * ln(p_i / q_i)
```
- **Alvo:** `PSI ≤ 0.2` (24h). Hook: `rollback_model`.

### E.2 KS (Kolmogorov–Smirnov)
- `KS ≤ 0.1` em score de aplicação; janela móvel 24h.

### E.3 SRM (A/B)
- Teste χ² para *Sample Ratio Mismatch*; **gate** bloqueia quando `p<0.01`.

## F) Observabilidade — OTel *deep dive*
- **Spans canônicos:** `decision.core`, `auction.match`, `fx.router`, `oracle.fetch`, `cdc.reader`, `dbt.run`, `ml.infer`.
- **Atributos obrigatórios:** `trace_id`, `pack_id`, `hook_id`, `latency_ms`, `status`, `audit_id`, `pii=false`.
- **Baggage/headers:** injetar `x-pack-id`, `x-hook-id`, `x-slo-budget`.
- **Eventos:** `hook.trigger`, `rollback.apply`, `slo.violation`, `ab.srm.fail`.

## G) CI/CD — pipelines com *gates* de qualidade e segurança
### G.1 GitHub Actions (esqueleto)
```yaml
name: ci
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-python@v5
        with: {python-version: '3.11'}
      - name: Cache UV
        uses: astral-sh/setup-uv@v1
      - run: uv sync --frozen
      - run: make be.lint be.test data.contracts.verify watchers.dry hooks.dry evidence.publish
      - name: Gate A110
        run: ./ops/scripts/gate_a110.sh --require-green --timeout 900
      - name: SBOM & vulns
        run: ./infra/security/sbom_scan.sh
```
### G.2 Padrões de revisão
- Tripla revisão: **conteúdo** • **técnica/CI** • **conformidade/segurança**.
- *Canary* com *feature flags* + observabilidade dedicada.

## H) Make/CLI — comandos canônicos
```
make env.up           # sobe containers locais
make be.lint be.test  # backend
make data.dbt         # dbt: run + test
make ml.serve         # start Triton/TorchServe
make hooks.dry        # simula triggers A110
make watchers.dry     # valida watchers
make evidence.publish # agrega JSONs de evidence
```

## I) DevX — *devcontainers*, seeds e reprodutibilidade
- `devcontainer.json` com Python 3.11, Node 20, Kafka local e Iceberg.
- Seeds determinísticos; *golden datasets* versionados.
- Artefatos (*model_sha*, *contract_sha*, *trace_id*, *audit_id*) anexados a releases.

## J) Perf — carga e *budget decomposition*
```
p95_total = p95_edge + p95_api + p95_dec + p95_oracle + p95_ml + p95_data + p95_persist
```
- **k6** para *load*; *Saturation test* antes de GA; *tail latency* monitorada.

## K) Runbooks (copiáveis)
### K.1 CDC Lag > 120s
1) Confirmar em painel `cdc.lag` (p95) • 2) Checar DLQ • 3) Acionar hook `degrade_to_hot_table` • 4) Abrir ticket DATA • 5) Postmortem ≤ 48h.

### K.2 SRM Fail
1) Pausar experimento via *flag* • 2) Registrar `ab.srm.fail` • 3) Auditoria → owners ST/ML.

### K.3 Oracle Staleness
1) Ver `oracle.staleness_ms` • 2) Acionar `switch_to_twap_failover` • 3) Validar `fx_delta_benchmark`.

## L) Segurança & Privacidade (prático)
- **Chaves:** rotação ≤ 90d; *envelope encryption*; *just‑in‑time secrets*.
- **Políticas:** PII fora de logs; *retention* mínima; *need‑to‑know*.
- **Supply chain:** SBOM, assinaturas, pinned digests.

## M) SDKs de referência
### M.1 TypeScript
```ts
import { decide } from "@creditengine/sdk";
const r = await decide({ app_id: "a-123", product: { code: "PX-001", amount: 5000, tenor_months: 12 } });
console.log(r.price.apr);
```
### M.2 Python
```py
from creditengine import decide
r = decide(app_id="a-123", product={"code":"PX-001","amount":5000,"tenor_months":12})
print(r["price"]["apr"])
```

## N) Padrões de nomenclatura & módulos
- **Eventos:** `domain.entity.action.vN` (`loan.application.created.v1`).
- **Recursos REST:** `kebab-case` nos paths; `snake_case` em payloads; `X-Feature-Flag` para *canary*.
- **Pacotes Python:** `creditengine.<domínio>.<módulo>`; *private* com sublinhado.

## O) ADRs e Waivers — formulários enxutos
```
ADR-YYYYMMDD-<slug>
Contexto • Requisitos/SLO • Opções • Decisão • Watchers • Hook A110 • Rollback • Owners • Evidências (links e hashes)
```

## P) Checklists “sem misericórdia” (prontas)
- [ ] Watchers verdes e **coverage=100%**
- [ ] Hooks A110 com **owner** e **rollback**
- [ ] Contratos (A106/A87/A89) publicados e versionados
- [ ] dbt tests verdes e `schema registry` sem incompat
- [ ] OTel presente (traces/spans/attrs)
- [ ] KPIs com *métrica→ação* clara
- [ ] AA/SRM test OK • Drift (PSI/KS) OK
- [ ] BCDR runbooks versionados
- [ ] Artefatos assinados (sha256) anexados

## Q) Exemplos de *hooks* A110 (YAML prontos)
```yaml
- hook: dec-latency-degrade
  kpi: dec.latency.p95
  threshold: 800ms
  window: 5m
  action: degrade_route
  owner: SRE
  evidence: traces:decision.core
  rollback: yes
- hook: oracle-stale-failover
  kpi: oracle.staleness_ms
  threshold: 30000
  window: 5m
  action: switch_to_twap_failover
  owner: BC
  evidence: audit:oracles/2025w36
  rollback: yes
```

## R) Postmortem (modelo 5‑porquês)
```
Resumo • Linha do tempo • Impacto • 5 Porquês • Ações (com dono e data) • Evidências • Status (overdue=0)
```

## S) Glossário expandido
A110 • ACE/DoR/DoD • SRM • PSI/KS • TWAP • CIP • CWV/INP • RPO/RTO • DLQ • Canary • Idempotency Key • Baggage • Burn Rate • SBOM • ETag • SR (sampling rate)

> **Pronto.** Este add‑on deixa o AGENTS.md no modo **Oscar**: denso, operacional, copiável, com tudo que o time precisa para codar, operar e auditar sem perder latência, controle ou governança.

