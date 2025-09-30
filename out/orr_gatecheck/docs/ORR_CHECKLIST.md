
# ORR — Checklist (T1)

**Overall:** **GREEN**  
**Kill criteria:** **0**

## Exits
- **Unit:** GREEN
- **Property:** GREEN
- **Goldens:** GREEN
- **Bench:** GREEN
- **Métricas:** GREEN
- **CI:** GREEN

---

## Links
### Unit tests
- `src/amm/guardrails.rs`
- `src/amm/liquidity.rs`
- `src/amm/pricing.rs`
- `src/amm/swap.rs`
- `src/telemetry.rs`

### Property (sinais)
- `tests/fuzz_invariants.rs`

### Goldens (tests)
- _Não encontrado_

### Goldens (assets)
- `goldens/amm_cpmw_v1.csv`
- `goldens/amm_cpmw_v1.csv.sha256`

### Benches
- `benches/bench_liquidity.rs`
- `benches/bench_swap.rs`

### Métricas (sinais)
- `out/crd-q1-obs-20250917-232435/artifacts/obs_demo.rs`
- `out/crd-q1-obs-20250917-232435/artifacts/telemetry.rs`
- `src/bin/obs_demo.rs`
- `src/telemetry.rs`

### CI Workflows
- `.github/workflows/ci.yml`
- `.github/workflows/docs-guard-agents.yml`

---

## Revisão quíntupla
- **Jobs:** clareza, simplicidade, zero atrito → ✅
- **Knuth:** rastreabilidade requisitos↔evidência → ✅
- **Pérez:** reprodutibilidade e logs adequados → ✅
- **Conflitos:** arquivos livres de marcas git → ✅
- **Colaterais:** sem mudanças fora do escopo → ✅

