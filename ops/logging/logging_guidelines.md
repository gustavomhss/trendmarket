# Resumo e Objetivo
Logs estruturados em JSON asseguram correlação determinística com traces (`trace_id` e `span_id`) e reduzem ambiguidade.
Todo evento DEVE ser compacto para controlar custo de ingestão e cardinalidade em Loki, mantendo observabilidade auditável.

# Contrato de Dados (v1)
O schema `ops/logging/log_schema.json` define o contrato v1 e NÃO DEVE ser violado. Campos obrigatórios (7):
- `ts`: instante UTC RFC3339 com sufixo `Z`.
- `level`: severidade (`TRACE|DEBUG|INFO|WARN|ERROR`).
- `msg`: mensagem principal (≤2048).
- `service`: emissor (`^[a-z0-9][a-z0-9-]{1,127}$`).
- `env`: ambiente (`dev|stg|prod`).
- `trace_id`: 32 hex minúsculos.
- `span_id`: 16 hex minúsculos.

# Labels (Loki) — Baixa Cardinalidade
Somente `service`, `env`, `op` e `level` DEVEM ser usados como labels fixas; `version` é opcional.
`trace_id` e `span_id` NÃO DEVEM entrar em labels para evitar explosão de cardinalidade e custos excessivos.

# Catálogo de `op` e `op_detail`
`op` é fechado: `swap`, `add_liquidity`, `remove_liquidity`, `pricing`, `cdc_consume`, `other`.
`op_detail` FICA no payload e DEVE refletir a granularidade relevante.
Exemplos recomendados:
- `swap`: `exact_in.single_hop`, `exact_in.multi_hop`, `exact_out.single_hop`.
- `add_liquidity`: `single_asset`, `dual_asset`, `scheduled.deposit`.
- `remove_liquidity`: `scheduled.withdrawal`, `instant.withdrawal`.
- `pricing`: `quote_only`, `slippage_check`, `twap_refresh`.
- `cdc_consume`: `partition.catchup`, `partition.replay`, `snapshot.load`.
- `other`: exigir `op_raw` com o valor bruto sanitizado (sem PII).

# PII & Segurança
É terminantemente proibido registrar `cpf`, `email`, `address`, `phone`, `user.id`, `user_id`, `authorization`, `set-cookie`,
`session`, `token`, `secret` ou `password`. O collector DEVE sanitizar antes de enviar.
Diante de incidente, DEVE-SE pausar exportação, abrir fluxo de deleção e publicar post-mortem com owners.

# Tamanho e Truncamento
`msg` NÃO DEVE exceder 2048 caracteres. Quando precisar truncar, aplicar sufixo `…[truncated]` e sinalizar
`msg_truncated=true`. Dados críticos DEVEM migrar para campos estruturados.

# Exemplos de Eventos
```json
{
  "ts": "2025-10-12T12:34:56Z",
  "level": "INFO",
  "msg": "Swap executado no par ce-amm.usdc-usdt com slippage controlada",
  "service": "ce-amm-router",
  "env": "prod",
  "version": "2025.10.1",
  "op": "swap",
  "op_detail": "exact_in.single_hop",
  "trace_id": "1f3a2b4c5d6e708192a3b4c5d6e70819",
  "span_id": "9abc0123def45678",
  "amm": {
    "k_before": 12345.67,
    "k_after": 12355.89,
    "delta_k_ratio": 0.0008
  },
  "msg_truncated": false
}
```
```json
{
  "ts": "2025-10-12T12:36:11Z",
  "level": "ERROR",
  "msg": "Pricing falhou por validação de parâmetros",
  "service": "ce-pricing-gateway",
  "env": "stg",
  "version": "2025.9.3",
  "op": "pricing",
  "op_detail": "slippage_check",
  "trace_id": "2a3b4c5d6e708192a3b4c5d6e708192a",
  "span_id": "abcd0123ef456789",
  "code": {
    "filepath": "pricing/slippage.rs",
    "lineno": 87
  },
  "error": {
    "kind": "validation",
    "code": "CE-PRC-VALIDATION-2001"
  },
  "msg_truncated": false
}
```
```json
{
  "ts": "2025-10-12T12:40:05Z",
  "level": "INFO",
  "msg": "CDC consumidor aplicou remapeamento de schema",
  "service": "ce-cdc-worker",
  "env": "dev",
  "version": "2025.10.0",
  "op": "other",
  "op_detail": "schema.adjustment",
  "op_raw": "cdc_rebalance_segment",
  "trace_id": "3b4c5d6e708192a3b4c5d6e708192a3b",
  "span_id": "bcde0123f4567890",
  "hook_id": "cdc-lag-guard",
  "msg_truncated": false
}
```

# Anti-padrões
Eventos DEVEM ser rejeitados quando contiverem PII, labels fora do catálogo permitido, dumps binários, ausência de `trace_id`
ou `span_id`, `env` fora de `dev|stg|prod` ou `level` em minúsculas. Logs fora do schema serão descartados.

# Boas Práticas por Ambiente
- **dev:** DEVE limitar DEBUG/TRACE a diagnósticos pontuais para evitar ruído crônico.
- **stg:** DEVE espelhar prod, validando hooks e watchers com volume próximo ao real.
- **prod:** PRIORIZE INFO/WARN; DEBUG apenas sob feature flag temporária e com expiração definida.

# Governança de Mudanças
Toda evolução do contrato DEVE seguir RFC curta, apontar impacto em queries Loki, labels e coletores.
Novas versões geram `logs.schema.v2.json`; versões anteriores NÃO DEVEM ser alteradas para preservar compatibilidade.

# Apêndice A: Taxonomia de Erros
`error.kind` DEVE usar a enumeração fechada. `error.code` segue `CE-<DOM>-<KIND>-<NNN>` com faixas: AMM 1000-1999, PRC
2000-2999, CDC 3000-3999, API 4000-4999, OBS 5000-5999, STO 6000-6999, SEC 7000-7999, INF 8000-8999, GEN 9000-9999.

# Apêndice B: Referências
- Especificação Ultra v1.3 (`specs/OBS-5/CRD-8-Ultra-v1.3.md`).
- Schema v1 (`ops/logging/log_schema.json`).
- Matriz de Testes v1.2 (`ops/tests/obs5_matrix_v1.2.md`).

Validação rápida:
- Validar eventos contra o schema com qualquer validador JSON Schema draft 2020-12 apontando para `ops/logging/log_schema.json`.
- Conferir labels permitidas revisando configuração Loki e filtros `service|env|op|level` (+`version` opcional).
- Identificar truncamento verificando `msg` com sufixo `…[truncated]` e `msg_truncated=true`.
