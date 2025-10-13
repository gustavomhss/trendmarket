# Loki Dev Configuration Specification (OBS-5 T4)

## Overview
Esta especificação descreve a configuração `ops/loki/loki.dev.yaml`, destinada ao ambiente de desenvolvimento local do OBS-5. O objetivo é habilitar ingestão do Collector (T3) para experimentação controlada, mantendo governança sobre cardinalidade e retenção para cumprir os SLOs do pacote.

## Blocos de configuração

### Autenticação e servidor
- `auth_enabled: false` mantém o servidor acessível somente em laço local sem delegar autenticação interna do Loki; a autenticação do stack OBS-5 é delegada a Grafana/RBAC quando integrado.
- `server.http_listen_address: 127.0.0.1` e `server.grpc_listen_address: 127.0.0.1` restringem o serviço ao loopback, prevenindo exposição indevida em DEV. A porta HTTP é fixada em `3100` para alinhar com o endpoint exigido pelo Collector. O gRPC (`9095`) permanece privado para componentes internos (frontend/querier).

### Common e ring
- `common.path_prefix: /loki` centraliza índices e chunks em um volume previsível para facilitar limpeza de ambiente.
- `common.replication_factor: 1` e `ring.kvstore.store: inmemory` evitam dependências externas e são adequados para uma instância única em DEV, mantendo semântica de ring compatível com produção.
- `common.storage.filesystem` aponta diretórios de chunks/regras em `/loki/*`, permitindo montar volumes efêmeros ou persistentes conforme necessário.

### Limites de ingestão e cardinalidade
- `ingestion_rate_mb: 6` limita throughput para ~50 Mbps, suficiente para testes de carga de T3 sem risco de saturar ambientes locais.
- `ingestion_burst_size_mb: 12` dobra temporariamente o teto para absorver rajadas curtas geradas pelos pipelines do Collector.
- `max_label_names_per_series: 20` garante aderência à política de baixa cardinalidade do OBS-5, impedindo que pipelines adicionem rótulos arbitrários.
- `max_label_value_length: 256` previne abusos (IDs longos ou payloads serializados) que degradam consultas e memória.
- `reject_old_samples: true` e `reject_old_samples_max_age: 168h (7 dias)` impedem carga retroativa que poderia corromper análises de teste, mantendo janelas alinhadas com SLOs semanais.
- `max_streams_per_user` e `max_global_streams_per_user` fixados em `5000` adicionam barreira extra para evitar explosão de séries em experimentos multi-tenant de DEV.

### Schema e storage
- `schema v13` com `boltdb-shipper` replica o padrão recomendado pela Grafana para Loki 2.9+, garantindo compatibilidade com features recentes e com o compactor nativo.
- `object_store: filesystem` com diretórios locais (`/loki/chunks`, `/loki/index`, `/loki/cache`) é suficiente em DEV, dispensando S3/GCS e simplificando setup offline.
- Índices com `period: 24h` equilibram granularidade e número de arquivos para retenção curta.

### Chunk store e retenção
- `chunk_store_config.max_look_back_period: 336h (14 dias)` define o horizonte máximo de consulta para duas semanas, coerente com cenários de debug sem acumular dados indefinidamente.
- O bloco `compactor` habilita retenção ativa, executando compações a cada 5 minutos e deletando dados após o período definido. O atraso de deleção (`2h`) dá margem para rollback manual, e o `delete_request_store: boltdb-shipper` mantém consistência com o backend escolhido.

### Ingester e caminho de escrita
- `ingester.wal.enabled: false` reduz uso de disco e é aceitável em DEV onde perda de dados é tolerável.
- Parâmetros de chunk (`chunk_idle_period`, `chunk_retain_period`, `chunk_block_size`, `chunk_encoding`) equilibram memória e compressão para o volume esperado de logs sintéticos.
- A lifecycler usa ring in-memory com `join_after: 0s` para inicialização rápida sem dependências externas.

### Consulta e caching
- `querier.engine.timeout: 1m` e `query_range.max_retries: 5` mantêm consultas responsivas, respeitando o objetivo de respostas em <1 s para janelas pequenas.
- `results_cache` com FIFO pequeno (1024 itens, validade 1 min) acelera repetição de queries exploratórias sem reter dados demais.
- `frontend`/`frontend_worker` ativam compressão de respostas e mantêm backlog controlado.

### Ruler e runtime
- O `ruler` utiliza armazenamento local e API habilitada para validar regras experimentais de alertas sem depender de Alertmanager externo. O endpoint é apontado para `127.0.0.1:9093`, previsto pelos pacotes observability.
- `runtime_config.file` referencia `/loki/runtime.yaml` para permitir ajustes dinâmicos futuros (por exemplo, limites temporários) sem alterar o arquivo versionado.

## Retention policy
A combinação de `max_look_back_period` (336h) com o compactor ativo estabelece retenção efetiva de 14 dias. Esse intervalo cobre investigações semanais e comparações quinzenais, enquanto evita acumular dados que prejudiquem performance ou compliquem descarte seguro em DEV.

## Endpoints de status
- `GET /loki/api/v1/status/buildinfo` confirma versão e build metadata do binário (Watcher `status_buildinfo_200`).
- `GET /loki/api/v1/status/ready` valida readiness geral após inicialização. Ambos devem responder `200` quando a instância está íntegra.

## Impacto de rejeição de amostras antigas
Ativar `reject_old_samples` garante que pipelines de teste não injetem eventos com timestamp defasado, o que poderia mascarar regressões reais ou drenar quota de retenção. Em conjunto com os limites de cardinalidade, isso mantém a série temporal limpa para ensaios de T5.

## Manifestos machine-readable

```yaml
ce-orr-obs5-t4:
  files:
    config: ops/loki/loki.dev.yaml
    spec: ops/loki/loki.dev.spec.md
  limits:
    ingestion_rate_mb: 6
    ingestion_burst_size_mb: 12
    max_label_names_per_series: 20
    max_label_value_length: 256
  retention:
    enabled: true
    look_back_hours: 336
    reject_old_samples_max_age_hours: 168
  schema:
    version: v13
    index_period_hours: 24
  acceptance:
    status_buildinfo_200: pending
```

```yaml
ce-orr-obs5-t4-env:
  mode: "direct_git"  # direct_git | offline_patch
  repo_base_branch: "main"
  create_branch: "obs5/t4-loki-dev"
```
