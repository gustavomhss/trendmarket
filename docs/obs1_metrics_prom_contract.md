# OBS-1 Prometheus Metrics Contract (`telemetry_metrics_prom`)

## Escopo e Objetivo
- Disponibiliza um servidor HTTP `/metrics` com formato Prometheus 0.0.4 para ambientes DEV/STG.
- Utiliza `opentelemetry-prometheus` para expor o `ManualReader` do pipeline de métricas e publica o resultado via HTTP leve (Hyper).
- O módulo é independente do exportador OTLP (T5), permitindo coexistência simultânea (collector OTLP + scrape Prometheus).

## APIs Disponíveis
| Função | Descrição |
| --- | --- |
| `init_prom_exporter() -> PromExporter` | Instala `prometheus::Registry` + `opentelemetry_prometheus::PrometheusExporter` sem abrir porta. Permite recuperar `meter_provider()` para registrar instrumentos. |
| `spawn_metrics_http(cfg: PromServerConfig, exporter: PromExporter) -> PromServerGuard` | Inicia o listener HTTP e retorna guard RAII (`Drop` = shutdown). Aceita `cfg.addr` (string) — use `0.0.0.0:9464` como default. |
| `PromServerGuard::shutdown(self)` | Encerramento assíncrono explícito. `Drop` também envia shutdown. |

## Fluxo de Inicialização
1. `let exporter = init_prom_exporter();`
2. `let provider = exporter.meter_provider();`
3. Registre instrumentos (histogramas, counters) antes de servir.
4. Leia `METRICS_HTTP_ADDR` (default `0.0.0.0:9464`) e chame `spawn_metrics_http`.
5. Mantenha o `PromServerGuard` vivo enquanto o endpoint estiver ativo.

### Execução DEV (exemplo canônico)
```bash
export PROM_SCRAPE=on
export METRICS_HTTP_ADDR=0.0.0.0:9464
# binário de demo (T12) chamará spawn_metrics_http(); aqui o contrato apenas mostra como o módulo é consumido
```

### Verificação manual
```bash
curl -sS http://127.0.0.1:9464/metrics | head -n 30
```

## Conteúdo da Resposta
- Status sempre `200 OK` em sucesso.
- Header obrigatório: `Content-Type: text/plain; version=0.0.4; charset=utf-8`.
- Corpo encodeado via `prometheus::TextEncoder`, incluindo séries `_bucket`, `_sum`, `_count`, `_total` conforme instrumentos registrados.
- Erros durante coleta/encode retornam `500` com mensagem textual.

## Coexistência com OTLP
- O módulo não configura OTLP automaticamente e não interfere no pipeline T5.
- É seguro executar simultaneamente `spawn_metrics_http` e o exporter OTLP: cada pipeline usa seu próprio `Reader`.
- Recomenda-se definir métricas comuns em ambos os pipelines para comparabilidade entre scrape e push.

## Toggle Operacional
- `PROM_SCRAPE=on` habilita o start no binário (controle externo a este módulo).
- `METRICS_HTTP_ADDR` define endereço/porta. Em ausência, usar `0.0.0.0:9464` conforme T2.

## Segurança e Rede
- Sem autenticação ou TLS neste estágio (escopo da thread). Utilize segmentação de rede ou sidecar para proteger o endpoint em ambientes compartilhados.

## Troubleshooting
| Sintoma | Ação recomendada |
| --- | --- |
| `Bind(...)` em `spawn_metrics_http` | Porta em uso. Ajuste `METRICS_HTTP_ADDR` ou encerre processo que monopoliza a porta. |
| Payload vazio | Certifique-se de registrar instrumentos antes de expor o endpoint e realizar medições (`add`/`record`). |
| Séries ausentes (`_bucket`/`_sum`/`_count`) | Verifique se o instrumento é do tipo histogram e se buckets foram preenchidos. |
| `Serve(...)` | Falha de I/O durante aceitação/serviço. Consulte logs e valide conectividade (firewall, proxies). |
| Performance | O loop aceita conexões sequencialmente; se necessário, ajuste o cliente de scrape para manter conexões curtas e monitorar latência. |

## Encerramento
- O `PromServerGuard` encerra o loop ao ser droppado ou via `shutdown().await`.
- Recomenda-se aguardar `shutdown().await` em testes para garantir liberação da porta antes de testes subsequentes.

## Compatibilidade
- Sem dependência de tracer/log (T4/T7).
- Pensado para ambientes DEV/STG; em PROD continue priorizando OTLP, mantendo `/metrics` opcional.
