# OBS-1 • Operação da Identidade de Serviço

## 1. Visão geral
O módulo `ServiceIdentity` concentra a resolução dos atributos canônicos para observabilidade. Utilize-o antes de inicializar traces, métricas ou logs para garantir `service.name`, `service.version` e `deployment.environment` consistentes.

## 2. Como influenciar a identidade
| Variável | Função | Observações |
| -------- | ------ | ----------- |
| `SERVICE_NAME` | Sobrescreve o nome padrão `ce-amm`. | Deve obedecer `^[a-z0-9._-]{3,64}$`. |
| `SERVICE_VERSION` | Define a versão diretamente. | Se ausente, a versão é composta com `CARGO_PKG_VERSION` + `CE_GIT_SHA_SHORT`. |
| `DEPLOY_ENV` | Ajusta o ambiente. | Aceita `dev`, `stg`, `prod` (case-insensitive). |
| `GIT_COMMIT` | Fornece o hash no CI quando `git` não está disponível. | Usado pelo `build.rs` para definir `CE_GIT_SHA`. |

O build script injeta automaticamente `CE_BUILD_TIME_RFC3339`, `CE_GIT_SHA` e `CE_GIT_SHA_SHORT`. Não há valores `unknown`.

## 3. Exemplos rápidos
### 3.1 Ambiente de desenvolvimento
```bash
export SERVICE_NAME=ce-amm
export DEPLOY_ENV=dev
# SERVICE_VERSION ausente → versão: "0.0.0+<git>"
```

### 3.2 Ambiente de staging
```bash
export SERVICE_NAME=ce-amm
export SERVICE_VERSION=2.3.1
export DEPLOY_ENV=stg
```

### 3.3 Ambiente de produção
```bash
export SERVICE_NAME=ce-amm
export DEPLOY_ENV=prod
# build.rs injeta CARGO_PKG_VERSION e CE_GIT_SHA_SHORT → versão "<semver>+<hash>"
```

## 4. Troubleshooting
| Sintoma | Ação recomendada |
| ------- | ---------------- |
| Build falha com `missing git sha (CE_GIT_SHA)` | Defina `GIT_COMMIT` no pipeline ou garanta que o diretório `.git` esteja disponível. |
| Erro `service.version ...` | Verifique se a string cumpre `^[A-Za-z0-9+._-]{2,64}$` **e** segue `MAJOR.MINOR.PATCH` ou inclui `+<hash>` com 7 hex. |
| Erro `build.time.utc ...` | O valor precisa terminar com `Z` e seguir RFC3339. Em builds locais, limpe diretórios e recompile para regenerar. |
| `deployment.environment` inválido | Ajuste `DEPLOY_ENV` para `dev`, `stg` ou `prod`. |

## 5. Consumindo os valores
```rust
use credit_engine_core::telemetry_identity::ServiceIdentityBuilder;

let identity = ServiceIdentityBuilder::new()
    .build()?; // utiliza envs + defaults
let attrs = identity.resource_pairs();
```
`attrs` já retorna os três pares para `Resource`. Use `identity.build_time_utc` e `identity.git_sha` em logs estruturados quando precisar de rastreabilidade adicional.
