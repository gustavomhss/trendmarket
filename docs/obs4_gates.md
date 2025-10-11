# OBS-4 — Quality Gates & Local Acceptance

Este guia descreve como executar localmente o gate OBS-4 criado para validar os critérios de Definition of Done: spans presentes, erro e lento amostrados, correlação disponível e configuração coerente do collector.

## Pré-requisitos

- Ambiente Unix-like com `bash` e `python3` disponíveis.
- Repositório `trendmarket` clonado e dependências Rust opcionais (o fluxo não exige build do projeto).
- Diretório `out/obs_gatecheck/` será criado automaticamente pelo gate.

## Comandos principais

1. **Rodar o gate completo**
   ```bash
   scripts/obs4_gate_local.sh
   ```
   O script executa em sequência:
   - Validação do collector (`Thread 02`) reutilizando `scripts/obs_t2_collector_dev.sh`.
   - Geração de amostras de spans e políticas (`Thread 07` – smoke).
   - Síntese JSON dos critérios de aceite (`Thread 07` – acceptance) via `scripts/obs4_gate_json.py`.

2. **Inspecionar o resumo JSON isoladamente**
   ```bash
   python3 scripts/obs4_gate_json.py
   ```
   Útil quando desejar reprocessar as evidências já geradas em `out/obs_gatecheck/evidence/`.

## Artefatos gerados

- `out/obs_gatecheck/logs/obs4_gate_local.txt`: log consolidado da execução.
- `out/obs_gatecheck/evidence/traces_sample.json`: amostra estruturada de spans, políticas e correlação.
- `out/obs_gatecheck/evidence/traces_raw.json`: índice bruto auxiliar (tamanho da amostra).
- Saída em stdout com `GATE=PASS` ou `GATE=FAIL:<motivo>` para fácil automação.

## Saída e códigos de retorno

- `GATE=PASS` + exit code `0`: todos os critérios validados.
- `GATE=FAIL:<motivo>` + exit code `3`: etapa específica falhou (`collector`, `smoke` ou `acceptance`).
- `scripts/obs4_gate_json.py` retorna `0` apenas quando `ok == true`; caso contrário, sai com `3` e imprime o resumo JSON com os flags.

## Troubleshooting rápido

- **Collector indisponível**: confirme se `ops/otel/collector-dev.rw.yaml` está íntegro; o runner tolera ausência do binário `otelcol` e registra a situação no log.
- **Amostra ausente**: reexecute `scripts/obs4_gate_local.sh` para regenerar `traces_sample.json`.
- **Resumo com `ok: false`**: verifique os campos `has_error`, `has_slow` e `has_links_cdc_amm` impressos pelo `obs4_gate_json.py` para identificar o critério pendente.

## Idempotência e reexecução

O gate pode ser executado múltiplas vezes; os arquivos em `out/obs_gatecheck/` são sobrescritos a cada rodada mantendo o diretório pronto para anexar evidências em PRs.

