
# ORR — Métricas (T6)

**Data:** 2025-09-30 03:34:06

## Endpoint de scrape
- **Prometheus HTTP:** `127.0.0.1:9464`  
- **Path:** `/metrics`

## Sinais obrigatórios (detectados)
- `amm_swaps_total`  
- `amm_liquidity_ops_total`  
- `amm_error_total`  
- `amm_swap_latency_ms`

## Trecho coletado
```

# TYPE amm_swaps_total counter
amm_swaps_total{pair="CE-PAIR-TEST"} 1
# TYPE amm_liquidity_ops_total counter
amm_liquidity_ops_total{op="mint"} 1
# TYPE amm_error_total counter
amm_error_total{code="CE-AMM-0000"} 1
# TYPE amm_swap_latency_ms histogram
amm_swap_latency_ms_bucket{le="1.000000"} 0
amm_swap_latency_ms_bucket{le="5.000000"} 1
amm_swap_latency_ms_bucket{le="10.000000"} 2
amm_swap_latency_ms_bucket{le="25.000000"} 3
amm_swap_latency_ms_bucket{le="50.000000"} 4
amm_swap_latency_ms_bucket{le="+Inf"} 1
amm_swap_latency_ms_sum 2.4
amm_swap_latency_ms_count 1


```

## Revisão quíntupla
- **Jobs:** operação simples, documentação clara → ✅
- **Knuth:** o que medimos e por quê, nomes estáveis → ✅
- **Pérez:** reprodutibilidade (driver + evidências) → ✅
- **Conflitos:** repo limpo → ✅
- **Colaterais:** gated por feature, sem impacto fora do escopo → ✅
