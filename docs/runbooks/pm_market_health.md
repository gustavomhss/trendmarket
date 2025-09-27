# Runbook PM — Mercados & Sinais

## oracle-divergence — `oracle_divergence_watch`
- **Hook:** `pm-oracle-staleness`
- **KPI:** `oracle.divergence_bps` ≤ 5 (janela 10m)
- **Ação automática:** `switch_to_twap_failover`
- **Owner:** PM/BC
- **Rollback:** quando divergência ≤ 2 bps por 3 janelas.

**Passos**
1. Checar painel `oracles.staleness` e logs de TWAP.
2. Validar feeds alternativos (CLS, CME) e registrar carimbos de tempo.
3. Sincronizar com DEC para confirmar que preços degradados foram aplicados.
4. Abrir ticket `PM-MKT` com análise de causa e anexar comparação de preços.

## fx-delta — `fx_delta_benchmark_watch`
- **Hook:** `fx-delta-benchmark-guard`
- **KPI:** `fx.delta_vs_benchmark_bps` ≤ 15 (janela 10m)
- **Ação automática:** `switch_to_reference_rate`
- **Owner:** PM FX
- **Rollback:** após delta ≤ 8 bps por 4 janelas.

**Passos**
1. Confirmar fonte de benchmark (ECB, BCB) e latência.
2. Revisar filas de roteamento FX e spreads.
3. Validar se experimentos abertos afetam spreads.
4. Atualizar canal #pm-markets com status.

## auction-invariant — `auction_invariant_breach_watch`
- **Hook:** `auction-invariant-pause`
- **KPI:** `auction.kkt_violation_pct` = 0 (janela 5m)
- **Ação automática:** `pause_auction_stream`
- **Owner:** PM Leilões
- **Rollback:** após auditoria matemática e validação de curvas.

**Passos**
1. Avaliar logs de `auction.match` para violações.
2. Executar script de verificação de convexidade (`scripts/check_convexity.py`).
3. Confirmar com DATA se há inconsistências de input.
4. Só retomar streaming após aprovação conjunta PM/DATA/SEC.

## slo-burn — `slo_budget_breach_watch`
Seguir plano descrito em `docs/runbooks/dec_decision_pricing.md#slo-burn` adicionando validações de mercado.
