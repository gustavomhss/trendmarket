# OBS-5 T3 — OTel Collector (Logs) Configuration Specification

## 1. Escopo e Objetivo
Esta especificação descreve a configuração `ops/otel/collector-logs.yaml`, dedicada exclusivamente ao pipeline de **logs**, conforme Especificação Ultra v1.3 e T1 v1.4. O objetivo é receber eventos via OTLP (gRPC/HTTP), remover PII antes da exportação, normalizar campos essenciais e entregar os lotes para Loki aplicando apenas labels de baixa cardinalidade, mantendo o endpoint de saúde `/healthz` disponível.

## 2. Extensões
- `health_check`: habilita monitoramento nativo do Collector, respondendo em `/healthz` na porta padrão 13133. Permite gates externos validarem a saúde do serviço sem depender de pipelines adicionais.

## 3. Receivers
- `otlp` com protocolos `grpc` e `http`: garante compatibilidade com emissores padronizados em OTLP, evitando bridges. Ambos permanecem com endpoints padrão para simplificar a integração e preservar determinismo.

## 4. Processors (ordem obrigatória)
1. **`memory_limiter`**: primeira etapa para proteger o Collector de picos, assegurando uso máximo de 75% de memória e absorvendo spikes até 15% adicionais. Evita backpressure precoce.
2. **`attributes/log_sanitize`**: executado logo após a limitação de memória para remover PII antes de qualquer enriquecimento. Trabalha sobre o conjunto de atributos bruto.
3. **`transform/ensure_trace_fields`**: normaliza campos funcionais (timestamp, mensagem e correlação). Executar após a sanitização evita reintroduzir dados sensíveis e garante que qualquer transformação opere sobre dados limpos.
4. **`batch`**: por último, agrega mensagens em lotes de até 8.192 itens ou 2s, equilibrando latência baixa com eficiência de rede/custo. Posicioná-lo no final garante que só dados já normalizados e seguros sejam agrupados.

### 4.1 Lista de PII (deny-list)
| Chave | Efeito |
| --- | --- |
| `cpf`, `email`, `address`, `phone` | Remove identificadores pessoais diretos. |
| `user.id`, `user_id` | Evita rastreabilidade individual. |
| `authorization`, `set-cookie`, `session`, `token`, `secret`, `password` | Elimina credenciais ou artefatos de sessão. |

## 5. Transformações
- **Timestamp (`ts`)**: quando ausente, o processador gera valor em UTC RFC3339 com `time_format(time_now(), "RFC3339")`, preservando ordenação temporal consistente.
- **Mensagem (`msg`)**: se a chave não existir e o corpo for string, replica-se o corpo para manter compatibilidade com consumidores que dependem de `msg`.
- **Correlação (`trace_id` / `span_id`)**: cria atributos derivados de `otel.trace_id` e `otel.span_id` apenas quando ainda não existem. Isso permite correlação downstream sem promover esses campos a labels (evitando explosão de cardinalidade) e mantém aderência à política da Especificação Ultra.

## 6. Exporter Loki
- Endpoint fixo `http://127.0.0.1:3100/loki/api/v1/push`.
- Labels permitidos: recursos (`service.name`, `deployment.environment`, `service.version`) e atributos (`level`, `op`). Apenas campos de baixa cardinalidade são expostos, minimizando custo de indexação.
- `default_labels_enabled.exporter=false` e `job=true` garantem nomes previsíveis sem ruído extra.
- `trace_id` e `span_id` permanecem fora das labels por exigência explícita para prevenir cardinalidade extrema e exposição de identificadores sensíveis.

## 7. Service
- `extensions: [health_check]`: ativa a checagem de saúde globalmente.
- `telemetry.logs.level: info`: garante visibilidade do ciclo de vida do Collector em produção.
- Pipeline único `logs` com recebedor OTLP, cadeia de processors na ordem mandatória e exportação final para Loki.

## 8. Fluxo de Dados (texto)
`OTLP receiver (gRPC/HTTP) -> memory_limiter -> attributes/log_sanitize -> transform/ensure_trace_fields -> batch -> Loki exporter`

Esta sequência assegura que o Collector trate limitações de recursos primeiro, limpe PII imediatamente, normalize campos críticos e somente então agrupe e envie logs sanitizados para Loki.
