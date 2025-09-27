# Runbook INT — Integrações & APIs

## api-breaking — `api_breaking_change_watch`
- **Hook:** `integration-contract-guard`
- **KPI:** `api.contract.breaking_changes` = 0 (janela 5m)
- **Ação automática:** `block_release`
- **Owner:** INT
- **Rollback:** após testes de contrato verdes e aprovação dos consumidores.

**Passos**
1. Executar `make be.contracts` e `make web.contracts` para reproduzir.
2. Checar versionamento de SDKs e comunicação aos clientes internos.
3. Ajustar payloads ou versionamento; publicar changelog.

## cache-ttl — `cache_ttl_misuse_watch`
- **Hook:** `integration-contract-guard`
- **KPI:** `integration.cache_ttl_violation_pct` = 0 (janela 10m)
- **Ação automática:** `block_release`
- **Owner:** INT/Platform
- **Rollback:** após correção de TTL e validação de headers.

**Passos**
1. Revisar configs de CDN/cache compartilhados.
2. Ajustar TTL conforme contrato e purgar caches inválidos.
3. Confirmar via logs que novos TTLs estão em vigor.

## cls-payin — `cls_payin_cutoff_watch`
- **Hook:** `integration-contract-guard`
- **KPI:** `integration.cls_payin_cutoff_delay_ms` ≤ 30000 (janela 5m)
- **Ação automática:** `block_release`
- **Owner:** INT/Payments
- **Rollback:** após automação normalizada (< 10.000 ms por 3 janelas).

**Passos**
1. Validar integrações com parceiros bancários e cron jobs.
2. Ativar manualmente o cutoff enquanto investiga atraso.
3. Comunicar Tesouraria e registrar ocorrência em `payments-incidents`.
