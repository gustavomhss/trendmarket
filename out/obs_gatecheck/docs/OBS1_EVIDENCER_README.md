# OBS-1 Evidencer README

## 1. Checklist rápido
1. Garanta `cargo` e `curl` instalados.
2. Execute o evidencer:
   ```bash
   scripts/obs_evidencer.sh --ops 8 --timeout-secs 30 --prom
   ```
3. Rode o teste de fumaça:
   ```bash
   tests/obs_evidencer_tests.sh
   ```
4. Verifique os artefatos em `out/obs_gatecheck/` e anexe ao PR.

## 2. Artefatos gerados
- `evidence/obs1_sdk.json` — manifesto JSON com métricas/logs/traces e `sha256` das fontes.
- `logs/obs1_smoke.txt` — stdout/stderr estruturado do `obs_demo`.
- `logs/obs1_metrics_sample.txt` — amostra do `/metrics` (presente quando `--prom`).

## 3. Troubleshooting
| Sintoma | Ação sugerida |
| --- | --- |
| Porta `127.0.0.1:9464` ocupada | Defina `METRICS_HTTP_ADDR="127.0.0.1:PORTA_LIVRE"` e re-execute com `--prom`. |
| `obs_demo` não encontrado | Rode a partir da raiz do repo (`trendmarket/`) para que `cargo run --bin obs_demo` funcione. |
| `curl: command not found` | Instale `curl` (ex.: `apt-get install curl`) ou execute sem `--prom` até que esteja disponível. |
| Script interrompe em `expected at least 3 non-zero buckets` | Aumente `--ops`, confirme que `PROM_SCRAPE=on` e verifique o exporter Prometheus. |

## 4. Como anexar no PR
1. Gere os artefatos com o evidencer.
2. Inclua no PR (ou em anexo) o conteúdo de:
   - `out/obs_gatecheck/evidence/obs1_sdk.json`.
   - `out/obs_gatecheck/logs/obs1_smoke.txt`.
   - `out/obs_gatecheck/logs/obs1_metrics_sample.txt` (quando existir).
3. Mencione na descrição do PR a data/hora da execução e a hash de commit em que foi gerada.
4. Confirme que `tests/obs_evidencer_tests.sh` passou localmente.

## 5. Referências
- Contrato completo: `docs/obs1_evidencer_contract.md`.
- Script: `scripts/obs_evidencer.sh`.
