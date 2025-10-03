# OBS-1 • Contrato de Logs JSON (Thread 7)

## Objetivo
Garantir que cada evento emitido pelo SDK siga o schema canônico definido para OBS-1, com correlação OpenTelemetry opcional, sem vazamento de PII e com níveis de log alinhados ao ambiente.

## Campos obrigatórios
Cada linha JSON **deve** conter os campos abaixo:

| Campo     | Tipo    | Descrição                                                                 |
|-----------|---------|----------------------------------------------------------------------------|
| `ts`      | string  | Timestamp UTC em formato RFC3339 (ex.: `2025-10-03T12:34:56Z`).            |
| `level`   | string  | Nível em minúsculas (`trace`, `debug`, `info`, `warn`, `error`).            |
| `msg`     | string  | Mensagem curta, sem placeholders.                                         |
| `service` | string  | Identificador do serviço emissor.                                         |
| `env`     | string  | Ambiente lógico (`dev`, `stg`, `prod`).                                   |
| `version` | string  | Versão semântica ou `git+<short_sha>`.                                    |

## Campos opcionais
Quando presentes, os campos abaixo devem seguir as regras estritas:

| Campo      | Tipo    | Regras                                                                                     |
|------------|---------|--------------------------------------------------------------------------------------------|
| `trace_id` | string  | Hexadecimal (32 chars) proveniente de span válido do tracer OTel.                          |
| `span_id`  | string  | Hexadecimal (16 chars) proveniente do span atual.                                          |
| `op`       | string  | Operação do domínio (`swap`, `add_liquidity`, `remove_liquidity`, `pricing`, `cdc_consume`).|
| Outros     | vários  | Qualquer atributo adicional deve evitar PII e placeholders; números e bools são preservados.|

## Proibições e saneamento
- Campos `email`, `cpf`, `phone`, `address`, `name`, `geo` ou prefixo `person_` são descartados automaticamente.
- Valores `"TBD"`, `"FIXME"`, `"…"`, `"PLACEHOLDER"` são bloqueados e substituídos por `"[blocked]"` quando necessário.
- `env` é normalizado para minúsculas e validado contra `dev|stg|prod`.

## Correlação de trace
Quando o aplicativo registrar a layer de `tracing_opentelemetry`, o módulo injeta `trace_id` e `span_id` com base no span corrente. Sem tracer ativo os campos não aparecem e nenhuma exceção é lançada. O campo `op` é resolvido a partir do span atual (`info_span!("...")`) ou do evento.

## Exemplos
### Linha válida (com correlação)
```json
{
  "ts":"2025-10-03T12:34:56Z",
  "level":"info",
  "msg":"swap executed",
  "service":"ce-amm",
  "env":"dev",
  "version":"2.4.0+1a2b3c4",
  "op":"swap",
  "trace_id":"4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f",
  "span_id":"9a3b7c1d2e3f4a5b"
}
```

### Linha inválida (PII)
```json
{
  "ts":"2025-10-03T12:34:56Z",
  "level":"info",
  "msg":"swap",
  "service":"ce-amm",
  "env":"dev",
  "version":"2.4.0+1a2b3c4",
  "email":"cliente@exemplo.com"
}
```
> O campo `email` é removido pelo formatter e deve ser tratado antes de chegar ao log.

## Recomendações de nível por ambiente
- **DEV**: `info` para comportamento padrão; habilite `debug` apenas em sessões curtas.
- **STG**: `info` e `warn`, mantendo volume controlado para validar integrações.
- **PROD**: `warn` e `error` como padrão; utilize `info` apenas em janelas específicas acordadas com SRE.

## FAQ
**O que acontece se não houver tracer?** A linha permanece válida sem `trace_id`/`span_id`.

**Como definir `op`?** Adicione `op="swap"` ao `info_span!` ou ao evento; valores fora da lista são descartados.

**Posso incluir campos adicionais?** Sim, desde que não estejam na lista de PII proibida e não usem placeholders.

**Como validar localmente?** Execute `cargo test telemetry_logs_tests` e confira `out/obs_gatecheck/evidence/obs1_logs_report.json`.
