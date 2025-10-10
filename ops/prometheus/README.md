# Prometheus scrape configs — CE stack

## Visão geral
Os ambientes **DEV** e **PROD** compartilham a mesma estrutura de scrapes para garantir paridade de telemetria do CreditEngine. No DEV usamos `static_configs` apontando para os exporters locais, enquanto em PROD a seleção de endpoints usa `file_sd` para permitir rotação dinâmica das instâncias sem alterar o arquivo principal do Prometheus.

## Endpoints monitorados
- **ce-app** expõe métricas no `:9464`.
- **otelcol** expõe métricas no `:8888`.

Em DEV ambos são referenciados como `localhost` para simplificar o ambiente de engenharia. Em PROD, as listas de targets residem nos arquivos `targets-prod.json` e `targets-otelcol-prod.json`, respeitando o escopo RFC1918.

## Higiene de labels
Aplicamos `metric_relabel_configs` com `action: labeldrop` e regex `^(instance|pod|container|namespace|endpoint)$` para remover apenas os rótulos efêmeros gerados pelos orchestrators. Isso garante séries mais estáveis e evita que o label `le` (necessário para histogramas) seja removido.

## Paridade DEV ↔ PROD
Todos os scrapes carregam labels estáveis (`service`, `env` e `stack`) para permitir a correlação consistente de métricas, dashboards e alertas entre ambientes. O arquivo de PROD herda os mesmos nomes de jobs e regras (`rules/core.rules.yml`) garantindo equivalência operacional.

## Sanidade rápida
1. Valide a sintaxe das configurações: `promtool check config ops/prometheus/prometheus.dev.yml` e `promtool check config ops/prometheus/prometheus.prod.yml`.
2. Com o Prometheus em execução, confirme os targets via `curl -sS http://localhost:9090/api/v1/targets` (ajuste host/porta conforme o deployment seguro) para verificar se as instâncias foram descobertas.

