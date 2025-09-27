# Runbook ML — Model Lifecycle

## model-drift — `model_drift_watch`
- **Hook:** `ml-model-rollback`
- **KPI:** `model.psi` ≤ 0.2 (janela 24h)
- **Ação automática:** `rollback_model`
- **Owner:** ML
- **Rollback:** automático após PSI ≤ 0.12 por 2 janelas e validação offline.

**Passos**
1. Validar PSI/KS por segmento e confirmar se há issue de entrada (DATA).
2. Verificar experimentos ativos e toggles de modelo canário.
3. Caso rollback ocorra, confirmar disponibilidade do baseline e recalcular métricas.
4. Documentar comparação de distribuições e anexar no ACE.

## srm — `ab_srm_watch`
- **Hook:** `experiment-srm-guardrail`
- **KPI:** `experiment.srm_pvalue` ≥ 0.01 (janela 24h)
- **Ação automática:** `pause_experiment`
- **Owner:** ML Experimentos
- **Rollback:** retomar somente após SRM > 0.05 e auditoria estatística.

**Passos**
1. Rodar script `scripts/experiments/check_srm.py`.
2. Validar randomização e event tracking com FE/INT.
3. Emitir relatório de impacto e plano de correção.

## runtime-eol — `runtime_eol_watch`
- **Hook:** `runtime-eol-governance`
- **KPI:** `runtime.support_gap_days` = 0 (janela 7d)
- **Ação:** `schedule_runtime_upgrade`
- **Owner:** PLAT + ML
- **Rollback:** não aplicável (migração planejada).

**Passos**
1. Atualizar roadmap de upgrade e alinhar janela de manutenção.
2. Garantir testes regressivos e SBOM atualizados.
3. Comunicar stakeholders e publicar ADR/waiver se necessário.

## image-vuln — `image_vuln_regression_watch`
- **Hook:** `ml-model-rollback`
- **KPI:** `image.critical_vuln_count` = 0 (janela 24h)
- **Ação automática:** `rollback_model`
- **Owner:** SEC + ML Ops
- **Rollback:** após rebuild seguro e varredura limpa (Grype/Trivy).

**Passos**
1. Revisar pipeline de containerização e dependências afetadas.
2. Rebuild da imagem com patches aplicados.
3. Executar scanner e anexar relatório assinado.
