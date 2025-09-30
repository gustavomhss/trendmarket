# Prometheus — Scrape do credit-engine-core

## Alvo
- Endpoint HTTP exposto pelo exportador Prometheus
- Padrão: `127.0.0.1:9464/metrics` (configurar via `AMM_METRICS_ADDR`)

## Configuração de exemplo (`prometheus.yml`)
```yaml
scrape_configs:
  - job_name: 'credit-engine-core'
    static_configs:
      - targets: ['127.0.0.1:9464']
```

## Séries

* `amm_swaps_total{pair="PAIR_ID"}`
* `amm_liquidity_ops_total{op="mint|burn"}`
* `amm_error_total{code="CE-AMM-XXXX"}`
* `amm_swap_latency_ms` (histograma)
