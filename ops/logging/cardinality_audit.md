# OBS-5 • T6 — Auditoria de Cardinalidade (Logs → Loki)

## 1. Política de labels e orçamento
- **Labels permitidas**: `service`, `env`, `op`, `level`, `version` (opcional).
- **Labels proibidas**: `trace_id`, `span_id`.
- **Máximo de labels por série**: 5 (hard limit).
- **`env`**: valores aceitos `dev`, `stg`, `prod`.
- **`op`**: catálogo fechado `{swap, add_liquidity, remove_liquidity, pricing, cdc_consume, other}` (≤ 6 valores ativos).
- **`version`**: até 50 valores ativos por ambiente; versões antigas devem ser liberadas do emissor assim que obsoletas.

Qualquer violação deve gerar **fail** e abertura de follow-up para correção do emissor. Valores próximos ao limite (≥ 80% do orçamento) entram em observação.

## 2. Janela de auditoria
- **Escopo temporal**: 24 horas retroativas `[now-24h, now]` em UTC.
- **Escopo lógico**: séries de logs com `service` iniciando em `ce-` e `env ∈ {dev, stg, prod}`.
- Auditorias extraordinárias podem ampliar a janela (até 72h) quando o volume de tráfego é baixo; registrar a variação no log de auditoria.

## 3. Metodologia
1. **API `series` (preferencial)**
   - Chamar `GET /loki/api/v1/series?match[]={service=~"ce-.*",env=~"(dev|stg|prod)"}&start=<start>&end=<end>`.
   - Calcular:
     - `total_series`: cardinalidade do array retornado.
     - `max_label_names_per_series`: máximo de labels por série (cada elemento é um mapa chave/valor).
     - `label_value_counts[label]`: cardinalidade de valores por label (`service`, `env`, `op`, `level`, `version`).
     - `forbidden_labels_present`: interseção entre labels retornadas e `{trace_id, span_id}`.
   - Persistir o array bruto (ou versão redigida) em `out/obs_gatecheck/evidence/labels_series_snapshot.json`.

2. **Aproximações (fallback)**
   - Quando a API `series` não estiver disponível, usar consultas LogQL que enumerem valores:
     - `count_values` ou `sum by (...) (count_over_time(...))` conforme exemplos em `cardinality_queries.md`.
   - Documentar a origem dos dados, limitações e qualquer amostragem usada.
   - O snapshot deve indicar `status: "approximated"` e explicar a limitação.

3. **Tendência (WoW)**
   - Comparar o relatório atual (`cardinality_report.json`) com o último arquivo versionado.
   - `wow_growth_pct = (atual − anterior) / max(anterior, 1)` para `total_series`, `service.distinct` e `version.distinct`.
   - Referenciar o relatório anterior com `trend.prev_run_ref`.

## 4. Critérios de aprovação
- **Pass**: limites respeitados, nenhuma label proibida, crescimento ≤ 20% WoW.
- **Pass com alerta**: crescimento > 20% e ≤ 35%, ou valores ≥ 80% do orçamento. Registrar alerta em `verdict.alerts` e abrir follow-up.
- **Fail**: labels proibidas, `max_label_names_per_series > 5`, `op.distinct > 6`, `version.distinct > 50` ou crescimento > 35%. Abrir incidente (SRE) antes do merge.

## 5. Relatórios e evidências
- `out/obs_gatecheck/evidence/cardinality_report.json`: relatório machine-readable consolidado (estrutura na Seção 5 do superprompt).
- `out/obs_gatecheck/evidence/labels_series_snapshot.json`: snapshot de séries/labels.
- `out/obs_gatecheck/logs/cardinality_audit.txt`: log textual da execução (timestamp, janela, método, totais, alertas, limitações).

**Interpretação do `cardinality_report.json`:**
1. Confirmar se `verdict.status` é `pass`. Caso contrário, bloquear merge.
2. Se houver alertas, abrir issue de capacity/observabilidade e registrar ação corretiva.
3. Revisar `trend.wow_growth_pct` para confirmar manutenção dentro do orçamento. Crescimentos sustentados acima de 20% exigem plano de mitigação (ex.: consolidar valores de `version`, revisar catálogo de `op`).
4. Avaliar `notes` para contexto adicional (amostragem, gaps).

## 6. Procedimento operacional
1. Executar consultas (`cardinality_queries.md`).
2. Salvar evidências nos caminhos acima (UTF-8, newline final).
3. Atualizar `trend.prev_run_ref` com a data (YYYY-MM-DD) do relatório anterior.
4. Anexar relatório e log ao PR.
5. Caso haja alerta/fail, abrir follow-up (issue/ticket) antes da aprovação.

## 7. Governança e rastreabilidade
- Todos os commits devem citar `OBS-5 T6`.
- Watchers automáticos: `watcher.obs5.t6.report_shape`, `watcher.obs5.t6.forbidden_labels`, `watcher.obs5.t6.limits`, `watcher.obs5.t6.trend`.
- Gate só libera merge com status `pass` (ou `pass` + alertas documentados).

