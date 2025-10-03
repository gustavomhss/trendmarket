# OBS-1 — Contrato de spans `cdc.consume`

Este documento normativo descreve o contrato para spans de consumo CDC na aplicação OBS-1. O objetivo é garantir telemetria
consistente para monitorar vazão, atraso e saúde por stream/partição.

## 1. Nome do span e operação

- **Nome do span:** `cdc.consume`
- **Campo `op`:** `"cdc_consume"`

O nome é estático e obrigatório. O campo `op` permite correlacionar spans e logs.

## 2. Atributos obrigatórios

| Atributo             | Tipo    | Restrições                                                                 |
|----------------------|---------|----------------------------------------------------------------------------|
| `cdc.stream`         | string  | Regex `^[a-z0-9._-]{3,64}$`; minúsculas; sem espaços.                      |
| `cdc.partition`      | string  | Regex `^[a-zA-Z0-9._-]{1,32}$`; evita cardinalidade explosiva.            |
| `cdc.offset_before`  | int64   | `>= -1`; use `-1` apenas quando o offset inicial é desconhecido.          |
| `cdc.offset_after`   | int64   | `>= offset_before`.                                                        |
| `cdc.records`        | int64   | `>= 0`.                                                                    |
| `cdc.lag_seconds`    | double  | `>= 0`; deve ser número finito (sem `NaN`, `+∞` ou `-∞`).                  |

### 2.1 Regras de coerência

1. `offset_after >= offset_before`.
2. Se `records > 0`, então `offset_after - offset_before >= records` (mínimo avanço monotônico).
3. `lag_seconds` representa o atraso observado **no momento do consumo**.

Violação de qualquer regra deve abortar a criação do span com mensagem acionável.

## 3. Boas práticas de cardinalidade

- Reutilize identificadores estáveis para `stream` e `partition`.
- Evite incluir `tenant_id`, `request_id` ou UUIDs; use nomes curtos como `trades`, `balances_eu`, `p0`.
- Mantenha `partition` alinhado à nomenclatura do broker (`p0`, `p1`, `shard-1`).
- Ao versionar contratos, prefira `stream` diferentes (`trades.v2`) em vez de valores dinâmicos.

## 4. Exemplos

### 4.1 Válido

```rust
let attrs = CdcConsumeAttrs {
    stream: "trades".into(),
    partition: "p0".into(),
    offset_before: 1000,
    offset_after: 1042,
    records: 42,
    lag_seconds: 0.250,
};
let span = span_cdc_consume(&attrs);
```

### 4.2 Wrapper RAII

```rust
let out = in_cdc_consume(&attrs, || process_batch());
```

### 4.3 Inválidos

| Cenário                         | Motivo                                                                 |
|---------------------------------|------------------------------------------------------------------------|
| `stream = ""`                  | Falha na regex `^[a-z0-9._-]{3,64}$`.                                  |
| `offset_after < offset_before`  | Offsets regressivos quebram monotonicidade.                            |
| `records = -1`                  | Contador de registros deve ser não-negativo.                           |
| `lag_seconds = -0.1`            | Atraso não pode ser negativo.                                          |
| `lag_seconds = NaN`             | Valores não finitos são proibidos.                                     |

## 5. Troubleshooting

| Sintoma                                           | Causa provável                                     | Ação recomendada                                      |
|---------------------------------------------------|----------------------------------------------------|--------------------------------------------------------|
| Erro `invalid cdc.consume attribute \`cdc.stream\`` | Nome da stream fora do padrão ou vazio.            | Ajustar para formato `^[a-z0-9._-]{3,64}$`.            |
| Erro sobre `offset_after`                         | Offset final menor que o inicial.                  | Garantir commit monotônico antes de criar o span.      |
| Erro de `records`                                 | Contagem negativa ou inconsistência com offsets.   | Revisar cálculo; se `records>0`, avance offsets.       |
| Erro `lag_seconds`                                | Valor negativo ou não finito.                      | Verificar fonte de tempo; sanitize antes do span.      |
| Lag alto (> SLA)                                  | CDC atrasado.                                      | Consultar painel `cdc.lag`, acionar hook A110 se preciso. |

## 6. Referência rápida

- Span RAII: `span_cdc_consume(&attrs)` retorna `tracing::Span` com validade automática.
- Wrapper: `in_cdc_consume(&attrs, || {...})` executa a closure dentro do span.
- Falhas de validação causam `panic!` com mensagem clara para corrigir a entrada.

Mantenha este contrato sincronizado com as implementações e testes automatizados.
