# OBS-5 — LogQL Validation Matrix (TC-LOG-001…040)

## Contexto e Objetivo
Este guia normativo consolida as consultas LogQL e critérios aceitos para a Matriz de Testes OBS-5 (casos TC-LOG-001 até TC-LOG-040). O foco é validar ingestão, estrutura, integridade de labels, correlação com traces e SLOs operacionais do serviço **`ce-amm`** no ambiente **`dev`**. Todas as consultas devem ser executadas no Loki apontado pela stack OBS-5, com captura de evidências (JSON/PNG) referenciadas no manifesto `logs_pipeline.json`.

## Padrões de Execução
- Período de consulta padrão: janela relativa `now-15m` → `now`, salvo indicação contrária.
- Sempre filtrar por `service="ce-amm"` e `env="dev"` (mínimo). Ops adicionais conforme o caso.
- Exportar evidências usando `loki/api/v1/query_range` (ou UI) com `step` ≤ 5s para séries temporais.
- A cada teste, capturar o payload bruto (`resultType`, `values`/`streams`) e um resumo textual no log de smoke (`obs5_logs_smoke.txt`).
- Em regressões, anexar comparação com execução anterior (delta de pontos, labels, métricas).

## Casos de Teste
### TC-LOG-001 — Existência de eventos pós-tráfego
- **Consulta:** `{service="ce-amm", env="dev"}`
- **Procedimento:** Injetar tráfego por ≥5s; aguardar até 30s e executar consulta.
- **Critério de sucesso:** Retornar ≥1 stream com `values` recentes (<30s do fim do tráfego).
- **Evidência:** screenshot/JSON do Loki + linha correspondente no log de smoke.

### TC-LOG-002 — Filtro por operação swap e nível ERROR
- **Consulta:** `{service="ce-amm", env="dev", op="swap", level="ERROR"}`
- **Procedimento:** Executar em fluxo nominal.
- **Critério de sucesso:** Resultado vazio (0 streams). Em teste de erro forçado, deve retornar >0.
- **Evidência:** Captura do resultado vazio + evidência do cenário de erro se executado.

### TC-LOG-003 — Projeção JSON e formatação RFC3339
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "ts={{.ts}} trace={{.trace_id}} msg={{.msg}}"`
- **Procedimento:** Validar três amostras.
- **Critério de sucesso:** Cada linha exibe `ts` RFC3339Z, `trace_id` não vazio (32 hex) e `msg` presente.
- **Evidência:** Texto exportado + anotação do `trace_id` correlacionado em Tempo/Jaeger.

### TC-LOG-004 — Taxa de erros 5m por operação
- **Consulta:** `sum by (op) (rate(({service="ce-amm"} |= "\"level\":\"ERROR\"")[5m]))`
- **Procedimento:** Avaliar após tráfego.
- **Critério de sucesso:** Valor `0` para operações sem erro. Em injeção de falhas, taxa >0 documentada.
- **Evidência:** Série temporal com valores numéricos + comentário sobre o cenário.

### TC-LOG-005 — Auditoria de labels proibidas
- **Consulta:** `{service="ce-amm", env="dev"}` (listar metadata)
- **Procedimento:** Inspecionar `result[0].stream` no payload bruto.
- **Critério de sucesso:** Labels presentes ⊆ {`service`,`env`,`op`,`level`,`version`}; ausência de `trace_id`/`span_id` como label.
- **Evidência:** JSON do stream com anotação destacando o conjunto de labels.

### TC-LOG-006 — Distribuição de severidade (conteúdo JSON)
- **Consulta:** `sum by (level) (count_over_time({service="ce-amm", env="dev"}[5m]))`
- **Procedimento:** Executar após tráfego nominal.
- **Critério de sucesso:** Níveis esperados (`debug`, `info`, `warn`, `error`) aparecem conforme mix previsto (<5% warn/error).
- **Evidência:** Série agregada + cálculo percentual no log de smoke.

### TC-LOG-007 — Cobertura de operações principais
- **Consulta:** `sum by (op) (count_over_time({service="ce-amm", env="dev"}[10m]))`
- **Procedimento:** Confirmar que todas as operações canônicas (`swap`, `add_liquidity`, `remove_liquidity`, `pricing`, `cdc_consume`) registram eventos.
- **Critério de sucesso:** Cada operação retorna contagem ≥1.
- **Evidência:** Tabela exportada com contagens.

### TC-LOG-008 — Latência de ingestão (tempo entre ts e _time)
- **Consulta:** `{service="ce-amm", env="dev"} | json | unwrap ts | line_format "ingest_delta={{ sub .__timestamp__ .ts | toDuration }}"`
- **Procedimento:** Comparar `ts` com `__timestamp__` (tempo de ingestão Loki).
- **Critério de sucesso:** `ingest_delta` ≤ 2s p95. Registrar valor no manifesto.
- **Evidência:** Amostra de 10 linhas + cálculo p95 manual.

### TC-LOG-009 — Correlação trace_id ↔ Tempo
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "trace={{.trace_id}} span={{.span_id}}"`
- **Procedimento:** Selecionar um `trace_id` e abrir no Tempo/Jaeger.
- **Critério de sucesso:** Encontrar ≥1 span correspondente; registrar caminho (`service`/`operation`).
- **Evidência:** Screenshot ou nota no smoke log com URL do trace.

### TC-LOG-010 — Validação de version tag
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "version={{.version}}"`
- **Procedimento:** Amostrar 20 eventos.
- **Critério de sucesso:** `version` segue SemVer ou SHA conforme regex do contrato.
- **Evidência:** Lista dos valores + regex check.

### TC-LOG-011 — Verificação de hook_id em erros
- **Consulta:** `{service="ce-amm", env="dev", level="error"} |= "hook_id"`
- **Procedimento:** Forçar erro associado a hook.
- **Critério de sucesso:** Eventos com `hook_id` presente e pattern `^[a-z0-9]+([-_][a-z0-9]+)*$`.
- **Evidência:** JSON com campo validado.

### TC-LOG-012 — Verificação de ausência de PII (email)
- **Consulta:** `{service="ce-amm", env="dev"} |= "@"`
- **Procedimento:** Rodar após tráfego.
- **Critério de sucesso:** Resultado vazio. Caso contrário, abrir incidente RB-LOG-PII.
- **Evidência:** Resposta vazia + hash da verificação regex.

### TC-LOG-013 — Verificação de ausência de PII (CPF)
- **Consulta:** `{service="ce-amm", env="dev"} |= "cpf"`
- **Procedimento:** Procurar indicadores de CPF.
- **Critério de sucesso:** Resultado vazio.
- **Evidência:** Screenshot do resultado.

### TC-LOG-014 — Verificação de ausência de PII (telefone)
- **Consulta:** `{service="ce-amm", env="dev"} |= "phone"`
- **Procedimento:** Conferir menções a telefone.
- **Critério de sucesso:** Resultado vazio.
- **Evidência:** JSON de retorno (vazio) com anotação.

### TC-LOG-015 — Cardinalidade de labels
- **Consulta:** `label_values({service="ce-amm", env="dev"}, op)`
- **Procedimento:** Listar valores de `op`.
- **Critério de sucesso:** Apenas valores esperados; nenhuma label surpresa.
- **Evidência:** Lista exportada.

### TC-LOG-016 — Contagem rolling 1m
- **Consulta:** `sum(count_over_time({service="ce-amm", env="dev"}[1m]))`
- **Procedimento:** Monitorar volume em janelas curtas.
- **Critério de sucesso:** Contagem consistente com tráfego aplicado (documentar baseline no log).
- **Evidência:** Valor numérico + comparação com gerador.

### TC-LOG-017 — Percentual de erros vs total
- **Consulta:** `100 * sum(rate({service="ce-amm", env="dev", level="error"}[5m])) / sum(rate({service="ce-amm", env="dev"}[5m]))`
- **Procedimento:** Executar após smoke.
- **Critério de sucesso:** Percentual ≤1% em dev.
- **Evidência:** Valor calculado (mesmo se NaN → tratar como 0% se sem eventos).

### TC-LOG-018 — Detecção de WARN acima do limite
- **Consulta:** `sum by (op) (count_over_time({service="ce-amm", env="dev", level="warn"}[10m]))`
- **Procedimento:** Avaliar ruído de warn.
- **Critério de sucesso:** Cada op ≤3 warns/10m.
- **Evidência:** Tabela com contagens.

### TC-LOG-019 — Latência e2e com marcador synthetic
- **Consulta:** `{service="ce-amm", env="dev"} |= "synthetic"`
- **Procedimento:** Localizar eventos do job synthetic (usado na prova de latência).
- **Critério de sucesso:** Pelo menos um evento com `synthetic=true` e `latency_ms` < 2000 registrado na extra.
- **Evidência:** Registro + cálculo no log.

### TC-LOG-020 — Validação de `extra` sem chaves proibidas
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "extra={{.extra}}"`
- **Procedimento:** Amostrar 10 eventos com `extra`.
- **Critério de sucesso:** Nenhuma chave começando com `email|cpf|phone|address|name|geo|person_`.
- **Evidência:** Print com destaque das chaves.

### TC-LOG-021 — Resiliência a burst (rate 1m)
- **Consulta:** `max_over_time(sum(rate({service="ce-amm", env="dev"}[1m]))[15m:1m])`
- **Procedimento:** Executar durante burst.
- **Critério de sucesso:** Valor ≤ capacidade testada (documentada em runbook, ex: ≤800 l/s).
- **Evidência:** Valor máximo reportado.

### TC-LOG-022 — Gap detector (ausência de logs)
- **Consulta:** `absent_over_time({service="ce-amm", env="dev"}[2m])`
- **Procedimento:** Executar durante tráfego contínuo.
- **Critério de sucesso:** Resultado vazio (sem gaps >2m).
- **Evidência:** Print da ausência.

### TC-LOG-023 — Logs de inicialização
- **Consulta:** `{service="ce-amm", env="dev", op="startup"}`
- **Procedimento:** Reiniciar componente e capturar logs de boot.
- **Critério de sucesso:** Eventos com nível `info` ou `debug` documentando configuração.
- **Evidência:** Registro do boot + hash do commit carregado.

### TC-LOG-024 — Logs de shutdown gracioso
- **Consulta:** `{service="ce-amm", env="dev", op="shutdown"}`
- **Procedimento:** Executar stop controlado.
- **Critério de sucesso:** Evento `info` confirmando flush completo.
- **Evidência:** Texto do log final.

### TC-LOG-025 — Monitoramento de SLA de pricing
- **Consulta:** `{service="ce-amm", env="dev", op="pricing"} |= "sla_ms"`
- **Procedimento:** Verificar medição de SLA em payload.
- **Critério de sucesso:** Campo `sla_ms` < 800 e `latency_ms` < 800.
- **Evidência:** Evento exportado com valores destacados.

### TC-LOG-026 — Logs de circuito aberto
- **Consulta:** `{service="ce-amm", env="dev"} |= "circuit_open"`
- **Procedimento:** Forçar circuito aberto.
- **Critério de sucesso:** Evento `warn/error` com `hook_id` correspondente; contagem ≤1 por janela.
- **Evidência:** Registro + justificativa.

### TC-LOG-027 — Consistência de span_id (16 hex)
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "span={{.span_id}}"`
- **Procedimento:** Validar regex `[0-9a-f]{16}` em 20 amostras.
- **Critério de sucesso:** Todas as amostras aderem.
- **Evidência:** Lista com validação.

### TC-LOG-028 — Correlação com métricas (latency bucket)
- **Consulta:** `{service="ce-amm", env="dev"} |= "metric=latency_bucket"`
- **Procedimento:** Confirmar emissão de logs com bucket de latência.
- **Critério de sucesso:** Presença de campos `bucket_le` e `count` no JSON extra.
- **Evidência:** Evento detalhado.

### TC-LOG-029 — Verificação de request_id opcional
- **Consulta:** `{service="ce-amm", env="dev"} |= "request_id"`
- **Procedimento:** Certificar que `request_id` (quando presente) segue pattern UUID v4.
- **Critério de sucesso:** Regex `^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$`.
- **Evidência:** Amostra validada.

### TC-LOG-030 — Logs de CDC consumer
- **Consulta:** `{service="ce-amm", env="dev", op="cdc_consume"}`
- **Procedimento:** Capturar 5 eventos consecutivos.
- **Critério de sucesso:** Cada evento inclui deslocamento (`offset`) e partição.
- **Evidência:** JSON com campos `offset` e `partition` na seção `extra`.

### TC-LOG-031 — Deteção de retries
- **Consulta:** `{service="ce-amm", env="dev"} |= "retry"`
- **Procedimento:** Forçar retry controlado.
- **Critério de sucesso:** Evento `warn` com `retry_count` < limiar (≤3) e tempo exponencial registrado.
- **Evidência:** Print do evento + cálculo backoff.

### TC-LOG-032 — Confirmar ausência de stacktrace sem contexto
- **Consulta:** `{service="ce-amm", env="dev"} |= "Traceback"`
- **Procedimento:** Procurar dumps de stack.
- **Critério de sucesso:** Resultado vazio; se presente, verificar se inclui `hook_id` e `error.kind`.
- **Evidência:** Registro (vazio ou não) e ação corretiva.

### TC-LOG-033 — Sincronização de relógio (ts monotônico)
- **Consulta:** `{service="ce-amm", env="dev"} | json | unwrap ts | sort ts`
- **Procedimento:** Ordenar amostra de 50 eventos.
- **Critério de sucesso:** `ts` crescente; desvios ≤1s.
- **Evidência:** Lista ordenada + análise.

### TC-LOG-034 — Delta entre trace start e log emitido
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "trace={{.trace_id}} start={{.extra.trace_start_ms}} emit={{.extra.emit_ms}}"`
- **Procedimento:** Coletar 10 eventos com `trace_start_ms`.
- **Critério de sucesso:** `emit_ms - trace_start_ms` < 200 ms.
- **Evidência:** Tabela com cálculos.

### TC-LOG-035 — Enriquecimento de tenant (quando aplicável)
- **Consulta:** `{service="ce-amm", env="dev"} |= "tenant_id"`
- **Procedimento:** Validar que `tenant_id` está anonimizando (hash) e pattern `[a-f0-9]{32}`.
- **Critério de sucesso:** Todos os valores seguem pattern; ausência de IDs brutos.
- **Evidência:** Lista com validação.

### TC-LOG-036 — Logs de integração externa
- **Consulta:** `{service="ce-amm", env="dev"} |= "external_provider"`
- **Procedimento:** Forçar chamada externa monitorada.
- **Critério de sucesso:** Evento `info`/`warn` com `external_provider` ∈ lista aprovada.
- **Evidência:** Registro e comparação com whitelist.

### TC-LOG-037 — Observabilidade Synthetic Control
- **Consulta:** `{service="ce-amm", env="dev"} |= "synthetic_control"`
- **Procedimento:** Confirmar job synthetic ativo.
- **Critério de sucesso:** ≥1 evento por 10m com campo `synthetic_control=true`.
- **Evidência:** Série com timestamps espaçados ~10m.

### TC-LOG-038 — Validação de `msg` curta (<256 chars)
- **Consulta:** `{service="ce-amm", env="dev"} | json | line_format "len={{len .msg}}"`
- **Procedimento:** Amostrar 50 eventos e medir comprimento.
- **Critério de sucesso:** Todos ≤256 caracteres.
- **Evidência:** Lista com máximos registrados.

### TC-LOG-039 — Verificação de `env` consistente
- **Consulta:** `{service="ce-amm"} | json | line_format "env={{.env}}"`
- **Procedimento:** Verificar se não há vazamento de logs de outros ambientes.
- **Critério de sucesso:** Apenas `dev` (ou env em teste específico); divergência → incidente.
- **Evidência:** Amostra de 20 valores.

### TC-LOG-040 — Sincronização com watcher.obs5.t5.labels_policy
- **Consulta:** `{service="ce-amm", env="dev"}` (payload completo)
- **Procedimento:** Cruzar labels presentes com política do watcher.
- **Critério de sucesso:** `labels` = [`service`,`env`,`op`,`level`] (+`version` opcional) em todas as streams.
- **Evidência:** Relatório anexado ao manifesto + nota no smoke log.

---
**Nota:** Cada execução deve ser rastreada com `run_id` único e vinculada ao manifesto `logs_pipeline.json`, garantindo repetibilidade e auditoria completa.
