# OBS-5 T2 — Requisitos normativos do emissor de logs do App CE

## 1. Propósito
Implementar o emissor de logs dos serviços CreditEngine (CE) garantindo conformidade com o Contrato de Dados v1, correlação com tracing OpenTelemetry e políticas anti-PII. O emissor deve produzir eventos determinísticos, prontos para ingestão pelo collector OBS-5, sem necessidade de pós-processamento local.

## 2. Contrato de evento (v1)
Todo evento emitido **deve** conter os sete campos obrigatórios a seguir:
- `ts`: timestamp UTC em RFC3339 (sufixo `Z`).
- `level`: domínio `{TRACE,DEBUG,INFO,WARN,ERROR}` em caixa alta.
- `msg`: mensagem truncada em 2048 caracteres; excedente recebe sufixo `…[truncated]` e atributo booleano `msg_truncated=true`.
- `service`: nome lógico do serviço CE (ex.: `ce-amm`).
- `env`: ambiente `{dev,stg,prod}`.
- `trace_id`: 32 hex minúsculo proveniente do contexto ativo OTel.
- `span_id`: 16 hex minúsculo proveniente do contexto ativo OTel.

Campos adicionais mandatórios:
- `version`: espelha `SERVICE_VERSION`.
- `op`: operação categorizada conforme [Catálogo de operações](#5-cat%C3%A1logo-de-opera%C3%A7%C3%B5es).
- `op_detail`: detalhe do catálogo; quando `op="other"`, preencher `op_raw`.
- `resource`: objeto com atributos canônicos:
  - `service.name`
  - `deployment.environment`
  - `service.version`

Eventos **sem** `trace_id` ou `span_id` em `level="ERROR"` **não** devem ser enviados; incrementar métrica `obs5.emitter.missing_trace_error` e retornar erro controlado.

## 3. Política de truncamento e enriquecimento
- Medir `msg.len()` em caracteres Unicode (normalizados). Ao exceder 2048, truncar para 2048 incluindo o sufixo `…[truncated]`.
- Ao truncar, adicionar `msg_truncated=true`. Caso contrário, omitir o campo.
- Campos livres adicionais devem evitar alta cardinalidade; preferir contagens e códigos.

## 4. Correlação com tracing
1. Recuperar o contexto atual via API OTel (`Span.current()` ou equivalente).
2. Se o contexto não estiver válido (`trace_id`/`span_id` zero) e o nível solicitado for `ERROR`, rejeitar o envio com log interno WARN e métrica `obs5.emitter.missing_trace_error`.
3. Para níveis < ERROR sem contexto válido, emitir com campos `trace_id`/`span_id` vazios **apenas** quando exigido pelo produto; caso contrário, aplicar política de chamada do consumidor (recomenda-se não emitir). Nunca sintetizar IDs.
4. Propagar baggage relevante via campos adicionais só quando explicitamente autorizados pelo contrato T1.

## 5. Catálogo de operações
Os valores aceitos estão definidos em `ops/logging/op_catalog.yaml`. Regras:
- O valor de `op` deve estar no conjunto `{swap, add_liquidity, remove_liquidity, pricing, cdc_consume, other}`.
- `op_detail` precisa corresponder a uma entrada do catálogo para o `op` selecionado.
- Para eventos fora do catálogo, usar `op="other"` e preencher `op_raw` com string curta (≤64 chars) sem PII.
- `op_detail` nunca vira label; permanece no payload.

## 6. Política anti-PII
Negativo absoluto para os tokens: `cpf`, `email`, `address`, `phone`, `user.id`, `user_id`, `authorization`, `set-cookie`, `session`, `token`, `secret`, `password`. O emissor deve aplicar filtros/mascaras e evitar dumping de payloads request/response. Utilize hashes estáveis ou contagens quando necessário.

## 7. Variáveis de ambiente obrigatórias
| Variável | Exemplo | Uso | Falha |
| --- | --- | --- | --- |
| `SERVICE_NAME` | `ce-amm` | Preenche `service` e `resource.service.name`. | Fail-fast com mensagem "missing SERVICE_NAME" e métrica `obs5.emitter.invalid_config`. |
| `SERVICE_VERSION` | `1.4.2` ou `git:abcd123` | Popula `version` e `resource.service.version`. | Idem acima. |
| `DEPLOY_ENV` | `dev`/`stg`/`prod` | Popula `env` e `resource.deployment.environment`. | Idem acima. |
| `OTLP_ENDPOINT` | `http://127.0.0.1:4318` | Destino OTLP (HTTP/protobuf ou gRPC). | Fail-fast, sem tentativa de fallback silencioso. |
| `RUNTIME_LOG_LEVEL` (ou equivalente) | `info` | Controla verborragia TRACE/DEBUG apenas em dev. | Warning + default para `info`. |

A inicialização deve validar as variáveis e interromper a aplicação antes de aceitar tráfego quando qualquer uma estiver ausente/invalidada.

## 8. Emissão OTLP
- Encaminhar eventos via exporter OTLP apropriado (HTTP ou gRPC) respeitando `OTLP_ENDPOINT`.
- Configurar batcher com flush ≤ 5 s e backoff exponencial (max 30 s) em falhas.
- Em caso de falha de rede, aplicar retry com jitter; ao estourar `max_retries`, registrar WARN mas não incluir payload sensível.

## 9. Métricas mínimas
- `obs5.emitter.sent_total` (counter por nível).
- `obs5.emitter.dropped_total` (counter com motivo: `missing_trace`, `invalid_contract`, `pii_detected`, `config_missing`).
- `obs5.emitter.msg_truncated_total` (counter).
- `obs5.emitter.queue_latency_ms` (histograma, objetivo p95 ≤ 2000 ms).

## 10. Testes e validações
1. Validar eventos contra `ops/logging/log_schema.json` (Contrato T1).
2. Executar scanner regex anti-PII sobre `samples/*.json` conforme lista da seção 6.
3. Confirmar presença de `trace_id` e `span_id` na amostra válida e ausência em campos obrigatórios da amostra inválida.
4. Exercitar cenários de truncamento, falha de variáveis obrigatórias e ausência de trace em níveis ERROR.

## 11. Exemplos
### 11.1 Evento válido
```json
{
  "ts": "2025-10-12T21:03:45Z",
  "level": "INFO",
  "msg": "swap executed",
  "service": "ce-amm",
  "env": "dev",
  "version": "1.4.2",
  "op": "swap",
  "op_detail": "exact_in.multi_hop",
  "trace_id": "8b2f4e0d3d2a4f79b1c2a3d4e5f6a7b8",
  "span_id": "1a2b3c4d5e6f7a8b",
  "resource": {
    "service.name": "ce-amm",
    "deployment.environment": "dev",
    "service.version": "1.4.2"
  }
}
```

### 11.2 Evento inválido (falta `span_id`)
```json
{
  "ts": "2025-10-12T21:03:45Z",
  "level": "INFO",
  "msg": "swap executed",
  "service": "ce-amm",
  "env": "dev",
  "version": "1.4.2",
  "op": "swap",
  "op_detail": "exact_in.multi_hop",
  "trace_id": "8b2f4e0d3d2a4f79b1c2a3d4e5f6a7b8",
  "resource": {
    "service.name": "ce-amm",
    "deployment.environment": "dev",
    "service.version": "1.4.2"
  }
}
```

## 12. Operação e governança
- Watchers obrigatórios: `watcher.obs5.t2.contract`, `watcher.obs5.t2.pii_scan`, `watcher.obs5.t2.catalog_shape`, `watcher.obs5.t2.resource_attrs`.
- Integrar com hooks A110 relevantes para queda de contrato ou violação de PII.
- Retenção: seguir política CE (mínimo necessário) e descarte seguro após ingestão.

## 13. Troubleshooting rápido
| Sintoma | Ação |
| --- | --- |
| `missing SERVICE_NAME` | Corrigir var e reiniciar; verificar secret/env vault. |
| `missing trace context` em ERROR | Validar instrumentação OTel, garantir spans ativos. |
| `collector 4xx` | Revisar contrato e catálogo; conferir schema. |
| `msg_truncated` crescendo | Revisar geração de mensagens; substituir payload por hash ou ID. |

## 14. Referências
- Especificação Ultra v1.3 (canônica).
- T1 v1.4 — Contrato & Guidelines.
- OBS-5 Livro de Operações — seção de logging.

## 15. Blocos YAML de rastreabilidade
```yaml
ce-orr-obs5-t2:
  files:
    guide: ops/logging/app_emitter_requirements.md
    catalog: ops/logging/op_catalog.yaml
    samples:
      - samples/t2_emit_valid_event.json
      - samples/t2_emit_invalid_event.json
  contract:
    required_fields: ["ts","level","msg","service","env","trace_id","span_id"]
    level_domain: ["TRACE","DEBUG","INFO","WARN","ERROR"]
  resource_attributes:
    - service.name
    - deployment.environment
    - service.version
  pii_policy:
    denylist: ["cpf","email","address","phone","user.id","user_id","authorization","set-cookie","session","token","secret","password"]
  acceptance:
    samples_validated: pending
    pii_matches: 0
```

```yaml
ce-orr-obs5-t2-env:
  mode: "direct_git"  # direct_git | offline_patch
  repo_base_branch: "main"
  create_branch: "obs5/t2-app-emitter"
```
