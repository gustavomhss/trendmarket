# OBS-1 — Contrato de Medição de Latência (`telemetry_latency`)

Este documento estabelece o padrão operacional para uso do módulo `telemetry_latency` (Thread 9 / CRD-8 OBS-1). O objetivo é medir a latência de operações críticas e registrar no histograma canônico `amm_op_latency_seconds` sem duplicar lógica de cronômetros no código de negócio.

## 1. Tipos expostos

- `Label { key, value }` — representa um rótulo validado.
- `LatencySink` — *trait* que recebe a duração (em segundos) e a lista de labels.
- `LatencyGuard` — RAII guard que mede a operação e publica na queda (`Drop`).
- Funções utilitárias: `with_latency`, `guard`, `is_valid_label_key`, `is_valid_label_value`.

## 2. Formatos e validação de labels

| Chave    | Regex / Valores permitidos                  | Obrigatório | Observações |
|----------|---------------------------------------------|-------------|-------------|
| `op`     | `^(swap|add_liquidity|remove_liquidity|pricing|cdc_consume)$` | Sim | Injetado automaticamente a partir do parâmetro `op` das funções públicas.|
| `service`| `^[a-z0-9._-]{3,64}$`                       | Recomendado | Identifica o serviço chamador; usar nomes canônicos (`amm.core`, `fx.router`, etc.). |
| `env`    | `dev`, `stg`, `prod`                        | Recomendado | Mantém cardinalidade controlada. |
| `version`| `^[A-Za-z0-9+._-]{2,64}$`                   | Opcional    | Use release/tag real. |

Qualquer chave fora da lista resulta em `LatencyError::ForbiddenLabel`. Valores fora do padrão também são rejeitados: nada de `tbd`, `unknown` ou placeholders similares.

## 3. Estilos de uso

### 3.1 Função wrapper

Use quando a operação cabe em um closure simples:

```rust
let labels = vec![
    Label::new("service", "amm.core"),
    Label::new("env", "prod"),
    Label::new("version", "2025.09.1"),
];
let result = with_latency("swap", &labels, &sink, || execute_swap(order));
```

O valor retornado por `with_latency` é o retorno do closure. A medição começa antes da execução e é registrada ao final (mesmo se o closure panic? => a duração ainda é enviada durante o `Drop` do guard interno).

### 3.2 RAII Guard

Útil para escopos maiores, múltiplos retornos ou estruturas complexas:

```rust
let labels = [Label::new("service", "pricing"), Label::new("env", "stg")];
let _guard = guard("pricing", &labels, &sink);
// ... lógica a ser medida ...
```

Ao sair do escopo (normal ou por panic), o `Drop` envia a duração para o `LatencySink` associado.

### 3.3 `LatencyGuard::new`

Para cenários em que queremos tratar falhas de validação manualmente, use `LatencyGuard::new(...) -> Result`. O wrapper `guard(...)` faz `panic!` em caso de erro para manter API enxuta.

## 4. Exemplos normativos

### 4.1 Invocação válida (`with_latency`)

```rust
let labels = vec![
    Label::new("service", "amm.core"),
    Label::new("env", "prod"),
];
let price = with_latency("swap", &labels, &sink, || quote_price(input));
```

### 4.2 Invocação válida (`guard`)

```rust
let labels = [Label::new("service", "amm.core"), Label::new("env", "stg")];
let _g = guard("pricing", &labels, &sink);
// cálculo extenso
```

### 4.3 Invocação inválida (será rejeitada)

```rust
let labels = [Label::new("team", "risk")]; // chave proibida
let _g = guard("swap", &labels, &sink); // panic! em runtime
```

Outro caso inválido:

```rust
let labels = [Label::new("service", "Amm-Core")]; // valor não corresponde ao regex
LatencyGuard::new("swap", &labels, &sink)?; // retorna Err(LatencyError::ForbiddenLabel(_))
```

## 5. Performance e boas práticas

- O cronômetro usa `Instant::now()` (monotônico) e converte para segundos com `as_secs_f64()`, garantindo precisão adequada para histogramas.
- O overhead é mínimo (alocação de `Vec<Label>` + clonagem dos labels fornecidos). Evite construir labels dentro de loops apertados; reutilize slices pré-alocados sempre que possível.
- Prefira manter `base_labels` imutáveis e compartilhadas (`lazy_static!`, `OnceLock`, etc.).
- O `LatencySink` é desacoplado: a Thread 8/T12 vai fornecer a implementação concreta conectada ao OTLP. Em testes, use um sink in-memory.

## 6. Troubleshooting

| Sintoma | Diagnóstico | Ação |
|---------|-------------|------|
| `panic!` com "latency guard error" | Label inválido ou `op` fora do contrato | Corrija a chave/valor; use `LatencyGuard::new` em caminhos sensíveis para propagar erro. |
| Métricas sem label `op` | Uso incorreto do sink | Nunca chame `LatencySink::record` diretamente; sempre passe pelo guard ou wrapper. |
| Cardinalidade explodindo | `service`/`version` com valores dinâmicos | Normalize: use valores fixos (ex.: `amm.core`, `2025.09.1`). |

## 7. Integração com outras threads

- T8/T12 devem implementar `LatencySink` para enviar dados ao histograma `amm_op_latency_seconds`.
- T10/T7 podem correlacionar spans/logs usando os mesmos labels.
- Este módulo não depende de `opentelemetry`; basta injetar o sink adequado.

---

**Checklist interno (Thread 9)**

- [x] Validação rígida de labels (`op`, `service`, `env`, `version`).
- [x] Medição por `Instant` + RAII guard.
- [x] Wrapper `with_latency` exposto.
- [x] Documentação e exemplos em conformidade com OBS-1.
