# Runbook SEC/PRIV — Segurança & Privacidade

## dep-vuln — `dep_vuln_watch`
- **Hook:** `sec-privacy-freeze`
- **KPI:** `security.deps.critical_vulns` = 0 (janela 15m)
- **Ação automática:** `freeze_release`
- **Owner:** Security Engineering
- **Rollback:** após mitigação e varredura limpa.

**Passos**
1. Consultar SBOM e scanner (Grype/Snyk) para lista de CVEs.
2. Aplicar patch/upgrade ou waiver com aprovação CISO.
3. Rodar `make security.scan` e anexar relatório.

## image-vuln — `image_vuln_regression_watch`
Seguir instruções do `docs/runbooks/ml_model_ops.md#image-vuln` com foco em supply chain.

## dp-budget — `dp_budget_breach_watch`
- **Hook:** `sec-privacy-freeze`
- **KPI:** `privacy.dp_budget_multiplier` ≤ 1.5 (janela 60m)
- **Ação automática:** `freeze_release`
- **Owner:** Privacy Engineering
- **Rollback:** após budget ≤ 1.2 por 2 janelas e revisão de auditoria.

**Passos**
1. Analisar logs de consultas DP e auditorias.
2. Revisar ruído/epsilon aplicado e pausar cargas de alto custo.
3. Documentar ajuste e liberar somente após aprovação do DPO.

## idp-keys — `idp_keys_rotation_watch`
- **Hook:** `sec-privacy-freeze`
- **KPI:** `security.idp.keys_age_days` ≤ 90 (janela 1d)
- **Ação automática:** `freeze_release`
- **Owner:** Security Engineering
- **Rollback:** não aplicável.

**Passos**
1. Agendar rotação com IAM Ops e registrar mudança.
2. Atualizar secrets, revogar chaves antigas e validar pipelines.
3. Monitorar autenticação por 24h para garantir ausência de erros.

## formal-verification — `formal_verification_gate_watch`
- **Hook:** `sec-privacy-freeze`
- **KPI:** `formal.verification.failure` = 0 (janela 5m)
- **Ação automática:** `freeze_release`
- **Owner:** Security Assurance
- **Rollback:** após verificação rerun aprovada.

**Passos**
1. Revisar logs do verificador (Coq/Z3) e identificar regressões.
2. Corrigir prova ou desabilitar mudança ofensora.
3. Retomar pipeline somente com aprovação dupla (SEC + domínio).
