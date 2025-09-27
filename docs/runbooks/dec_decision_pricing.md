# Runbook DEC — Decision & Pricing Core

Este runbook cobre os watchers de DEC definidos em `ops/watchers/core.yaml` e os respectivos hooks A110.

## metric-gap — `metrics_decision_hook_gap_watch`
- **Hook:** `dec-latency-degrade`
- **KPI:** `dec.latency.p95` ≤ 800 ms (janela 5m)
- **Ação automática:** `degrade_route`
- **Owner:** SRE
- **Rollback:** automático após estabilização (< 700 ms p95 por 2 janelas)

**Procedimento**
1. Verifique no painel `decision.core` se há saturação ou regressão recente.
2. Confirme traces com `trace_id` do período para garantir que o fallback aplicou.
3. Se o fallback não estabilizar em 2 janelas, acione `plat@creditengine` e prepare a reversão manual da última alteração.
4. Documente no ACE o impacto e anexos de traces.

## slo-burn — `slo_budget_breach_watch`
- **Hook:** `platform-sampling-guard`
- **KPI:** `slo.burn_rate` ≤ 1.0 (janela 60m)
- **Ação automática:** `block_release`
- **Owner:** SRE
- **Rollback:** após burn rate < 0.8 por 3 janelas.

**Procedimento**
1. Avalie dashboards de erro/latência para identificar o serviço infrator.
2. Habilite `feature flags` de degrade (ex.: `dec.degrade_to_baseline`).
3. Valide se os hooks dependentes (PM/PLAT) estão sincronizados.
4. Documente no runbook global de incidentes com timeline em até 4h.
