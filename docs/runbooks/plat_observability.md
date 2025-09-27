# Runbook PLAT — Observabilidade & Confiabilidade

## tracing-sampling — `tracing_sampling_watch`
- **Hook:** `platform-sampling-guard`
- **KPI:** `tracing.sampling_rate` ≥ 1% (janela 15m)
- **Ação automática:** `block_release`
- **Owner:** SRE
- **Rollback:** após sampling ≥ 3% por 2 janelas.

**Passos**
1. Checar config do Otel Collector e limites de ingestão.
2. Ajustar sampling dinâmico e confirmar em dashboards.
3. Reabrir deploy apenas após verificação cruzada com DEC/PM.

## alert-storm — `alert_storm_watch`
- **Hook:** `platform-sampling-guard`
- **KPI:** `alerts.per_minute` ≤ 50 (janela 10m)
- **Ação automática:** `block_release`
- **Owner:** SRE
- **Rollback:** quando taxa ≤ 30 por 3 janelas.

**Passos**
1. Identificar origem (ruído ou incidente) analisando tags.
2. Aplicar dedupe/quiet hours e alinhar com SecOps.
3. Registrar follow-up para ajustar regras de alerta.

## policy-violation — `policy_violation_watch`
- **Hook:** `platform-sampling-guard`
- **KPI:** `policy.violation_detected` = 0 (janela 5m)
- **Ação automática:** `block_release`
- **Owner:** SRE + Compliance
- **Rollback:** após waiver aprovado e remediação aplicada.

**Passos**
1. Verificar política quebrada (RBAC, CIP, etc.).
2. Aplicar mitigação e obter aprovação compliance.
3. Atualizar auditoria e checklist ACE.

## okr-alignment — `okr_risk_alignment_watch`
- **Hook:** `platform-sampling-guard`
- **KPI:** `okr.risk_alignment_score` ≥ 0.8 (janela 7d)
- **Ação automática:** `block_release`
- **Owner:** StrategyOps
- **Rollback:** após score ≥ 0.9 confirmado em review.

**Passos**
1. Revisar KPIs impactados e riscos abertos.
2. Preparar briefing executivo com ações corretivas.
3. Acompanhar implementação e atualizar status semanalmente.

## slo-burn — `slo_budget_breach_watch`
Seguir instruções de `docs/runbooks/dec_decision_pricing.md#slo-burn` com foco em capacidade e erros infra.
