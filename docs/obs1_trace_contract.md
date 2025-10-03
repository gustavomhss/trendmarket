# OBS-1 — Contrato de Tracing (`telemetry_trace`)

## Visão geral

O módulo `telemetry_trace` provê um inicializador autocontido para o provedor de tracing do OBS-1. Ele expõe uma API pura — sem efeitos globais além do registrador de propagadores — que devolve um `TraceGuard` (RAII) e uma `OpenTelemetryLayer<tracing_subscriber::Registry, opentelemetry_sdk::trace::Tracer>`. O chamador decide quando anexar a layer e quando desligar o provedor.

Principais requisitos atendidos:

- **Exportação OTLP** com `BatchSpanProcessor` e suporte a gRPC ou HTTP/protobuf.
- **Sampler configurável** conforme o nível de observabilidade acordado com o produto.
- **Propagadores W3C** (TraceContext + Baggage) registrados globalmente de forma idempotente.
- **Recurso canônico** composto por `service.name`, `service.version` e `deployment.environment` obrigatórios.
- **Shutdown limpo** garantido via `TraceGuard` — `Drop` chama `shutdown()` apenas uma vez.

## Estrutura de configuração

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceConfig {
    pub level: ObsLevel,
    pub otlp_endpoint: Option<String>,
    pub protocol: Option<OtlpProtocol>,
    pub export_timeout_ms: u64,
    pub max_queue_size: usize,
    pub scheduled_delay_ms: u64,
    pub max_export_batch_size: usize,
}
```

Valores padrão (`TraceConfig::default()`):

| Campo                    | Default | Observações                                                     |
|-------------------------|---------|-----------------------------------------------------------------|
| `level`                 | `ObsLevel::Off` | Define sampler e necessidade de endpoint                        |
| `otlp_endpoint`         | `None`  | Obrigatório quando `level` ∈ {`Min`, `Full`} em ambientes ativos |
| `protocol`              | `None`  | Quando `None`, aplica heurística `select_protocol`              |
| `export_timeout_ms`     | `10_000`| Timeout por lote em milissegundos                               |
| `max_queue_size`        | `2_048` | Tamanho máximo da fila do batch                                 |
| `scheduled_delay_ms`    | `5_000` | Intervalo entre exports                                         |
| `max_export_batch_size` | `512`   | Lote máximo enviado por ciclo                                   |

### Níveis de observabilidade

- `ObsLevel::Off` → Sampler `AlwaysOff`, provedor sem exportador, endpoint opcional (útil em dev/offline).
- `ObsLevel::Min` → `ParentBased(TraceIdRatioBased(0.01))` (1%): reduz custo mantendo rastreabilidade mínima.
- `ObsLevel::Full` → `ParentBased(AlwaysOn)`: coleta completa, indicado para produção.

### Resource (`ResourcePairs`)

O `ResourcePairs` deve conter **exatamente** três pares (`(&'static str, String)`):

1. `"service.name"`
2. `"service.version"`
3. `"deployment.environment"`

Valores vazios, chaves extras ou ausentes geram `TraceInitError::InvalidResource` com diagnóstico explícito.

## Seletor de protocolo (`select_protocol`)

Heurística aplicada quando `TraceConfig.protocol` é `None`:

1. Preferir valor explícito (`Some(OtlpProtocol)`)
2. Caso o endpoint contenha `:4318` **ou** termine com `/v1/traces` → `OtlpProtocol::Http`
3. Demais cenários → `OtlpProtocol::Grpc`

A heurística cobre as portas padrão (`4317` gRPC, `4318` HTTP) e os endpoints REST do Collector.

## Propagadores globais

Na primeira inicialização o módulo registra um `TextMapCompositePropagator` composto por:

- `TraceContextPropagator` (W3C TraceContext)
- `BaggagePropagator` (W3C Baggage)

O registro é **idempotente** e protegido por `OnceLock`. Chamadas subsequentes de `init_tracing` não panicarão. B3 não é habilitado por padrão para evitar ambiguidade com cabeçalhos legados; documente a ativação caso necessário.

## Exportação OTLP

- Exportador construído via `opentelemetry_otlp::SpanExporter::builder()`.
- Protocolos suportados: `grpc-tonic` (com TLS/Rustls habilitado) e `http-proto` (`reqwest` com Rustls WebPKI).
- `BatchSpanProcessor` configurado com `BatchConfig` derivado dos campos de `TraceConfig`.
- `TraceGuard::Drop` chama `shutdown()` e ignora `AlreadyShutdown`; demais erros são logados com `eprintln!`.

## Exemplos normativos

### Dev — modo Off (sem endpoint)
```rust
let cfg = TraceConfig {
    level: ObsLevel::Off,
    otlp_endpoint: None,
    protocol: None,
    export_timeout_ms: 10_000,
    max_queue_size: 2_048,
    scheduled_delay_ms: 5_000,
    max_export_batch_size: 512,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "0.0.0+devhash".into()),
    ("deployment.environment", "dev".into()),
];
let (guard, layer) = init_tracing(cfg, resource)?; // layer no-op
```

### Staging — OTLP gRPC explícito
```rust
let cfg = TraceConfig {
    level: ObsLevel::Min,
    otlp_endpoint: Some("http://otel-collector.stg.svc:4317".into()),
    protocol: Some(OtlpProtocol::Grpc),
    export_timeout_ms: 10_000,
    max_queue_size: 4_096,
    scheduled_delay_ms: 3_000,
    max_export_batch_size: 512,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "1.2.3-stg".into()),
    ("deployment.environment", "stg".into()),
];
let (guard, layer) = init_tracing(cfg, resource)?;
```

### Produção — OTLP HTTP autodetectado (porta 4318)
```rust
let cfg = TraceConfig {
    level: ObsLevel::Full,
    otlp_endpoint: Some("https://otel-collector.prod:4318/v1/traces".into()),
    protocol: None,
    export_timeout_ms: 10_000,
    max_queue_size: 8_192,
    scheduled_delay_ms: 2_000,
    max_export_batch_size: 512,
};
let resource = vec![
    ("service.name", "ce-amm".into()),
    ("service.version", "1.2.3".into()),
    ("deployment.environment", "prod".into()),
];
let (guard, layer) = init_tracing(cfg, resource)?;
```

### Casos inválidos

- Falta de `service.version` → `TraceInitError::InvalidResource("resource key 'service.version' is missing")`
- `ObsLevel::Full` com `otlp_endpoint: None` → `TraceInitError::MissingEndpointForActiveLevel`
- Endpoint malformado (ex.: `otel://collector`) → erro de construção `TraceInitError::OtlpBuildError(<detalhes do builder>)`

## Troubleshooting

| Sintoma                                   | Causa provável                               | Ação                                                                                 |
|------------------------------------------|-----------------------------------------------|--------------------------------------------------------------------------------------|
| `TraceInitError::InvalidResource`        | Chave ausente, duplicada ou valor vazio       | Verificar `ResourcePairs` — devem ser exatamente 3 entradas válidas                  |
| `TraceInitError::MissingEndpointForActiveLevel` | `level` ≠ `Off` sem endpoint configurado | Configurar `TraceConfig.otlp_endpoint` antes de chamar `init_tracing`                |
| `TraceInitError::OtlpBuildError`         | Endpoint inválido, TLS mal configurado        | Revisar URL, certificados (Rustls) e tempo limite                                    |
| Spans dropados em lote                   | Fila pequena ou `scheduled_delay` alto        | Ajustar `max_queue_size`, `max_export_batch_size` ou reduzir `scheduled_delay_ms`    |
| Timeout (`export_timeout_ms`) recorrente | Collector lento ou indisponível               | Revisar Collector, aumentar timeout ou aplicar fallback                              |
| Falha de TLS                             | Certificados ausentes/inválidos               | Habilitar `reqwest-rustls-webpki-roots` (já incluso) ou configurar `reqwest` custom   |

## FAQ

**Posso reutilizar o `TraceGuard` entre threads?** Sim — `SdkTracerProvider` é `Send + Sync`. Basta manter o guard vivo enquanto a layer estiver ativa.

**Preciso instalar o subscriber global dentro do módulo?** Não. A camada retornada deve ser adicionada pelo chamador (`tracing_subscriber::registry().with(layer)`), preservando separação das threads OBS-1.

**Como desligar manualmente?** Chame `guard.shutdown()` para flush explícito; ao sair de escopo `Drop` garante shutdown idempotente.

**B3 é suportado?** Não por padrão. B3 pode causar ambiguidades em ambientes híbridos; se necessário, adicione manualmente no chamador antes ou depois de `init_tracing`.

**Preciso de Collector em desenvolvimento?** Não. Use `ObsLevel::Off` ou forneça endpoint apontando para Collector local caso deseje spans reais.

## Garantias

- Registro de propagadores ocorre uma única vez, evitando panics em re-inicializações.
- `TraceGuard` sempre chama `shutdown()` e ignora `AlreadyShutdown` retornado pelo SDK.
- Configuração de sampler segue estritamente a tabela definida pelo produto (Off, Min, Full).
- Valores padrão refletem recomendações oficiais da especificação OTLP (timeout 10s, fila 2048, lote 512).

## Próximos passos

- Integração com `tracing`/logger (Thread 7) utilizará a layer retornada.
- Métricas e logs estruturados seguem nas threads T5 e T7, respectivamente.
