# OBS-5 • T6 — Consultas de Cardinalidade (Loki)

## 1. API `series` (preferencial)
```bash
START=$(date -u -d '24 hours ago' +%s)000000000
END=$(date -u +%s)000000000
curl -sS "https://<loki-host>/loki/api/v1/series" \
  --get \
  --data-urlencode 'match[]={service=~"ce-.*",env=~"(dev|stg|prod)"}' \
  --data-urlencode "start=${START}" \
  --data-urlencode "end=${END}" | jq '.'
```
- `start` e `end` em nanosegundos UNIX.
- Retorno esperado: array de objetos com labels. Persistir resposta em `labels_series_snapshot.json`.

## 2. Cardinalidade por label (LogQL) — fallback
> Usar apenas se a API `series` estiver indisponível. Registre a limitação no log da auditoria.

### 2.1 Distintos por `service`
```logql
sum by (service) (count_over_time({service=~"ce-.*", env="dev"}[5m]) > 0)
```
- Repetir por ambiente (`env="stg"`, `env="prod"`).
- Cardinalidade = número de séries com resultado > 0.

### 2.2 Distintos por `op`
```logql
count_values("op", {service=~"ce-.*", env="dev"})
```
- Ajustar `env` conforme necessário.
- Verificar se o conjunto de valores ⊆ catálogo permitido (`swap`, `add_liquidity`, `remove_liquidity`, `pricing`, `cdc_consume`, `other`).

### 2.3 Distintos por `level`
```logql
count_values("level", {service=~"ce-.*", env="dev"})
```
- Espera-se cardinalidade pequena (≤ 5). Valores inesperados (`fatal`, `notice`) devem ser auditados.

### 2.4 Combinação de labels (aproximação de séries)
```logql
sum by (service, env, op, level, version) (count_over_time({service=~"ce-.*", env=~"(dev|stg|prod)"}[5m]) > 0)
```
- Cardinalidade aproximada = número de linhas retornadas.
- Se `version` não estiver presente, remover do `sum by`.

### 2.5 Cardinalidade de `version`
```logql
count_values("version", {service=~"ce-.*", env="dev"})
```
- Repetir por ambiente e consolidar total ≤ 50.

## 3. Avisos importantes
- Priorize a API `series` para precisão: ela reflete exatamente as séries ativas no índice.
- Consultas de fallback podem superestimar ou subestimar cardinalidade (janelas de 5m, amostragem). Sempre registrar o método utilizado.
- Nunca criar labels novas no emissor para viabilizar auditoria; ajustes estruturais pertencem às tarefas T3/T4.
- Certifique-se de executar as consultas com credenciais de leitura e em ambiente equivalente ao auditado (dev/stg/prod) para evitar vazios.

