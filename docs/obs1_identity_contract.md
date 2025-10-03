# OBS-1 • Identidade de Serviço (Contrato Canônico)

## 1. Objetivo
Garantir que todas as superfícies de observabilidade (traces, métricas e logs) publiquem atributos canônicos, estáveis e auditáveis para o serviço **credit-engine-core**. Este contrato consolida as regras de resolução, precedência e validação da identidade de serviço consumida pelo módulo `ServiceIdentity`.

## 2. Atributos Canônicos
| Campo Resource | Descrição | Fonte padrão | Regex | Tamanho |
| -------------- | --------- | ------------- | ----- | ------- |
| `service.name` | Nome lógico do serviço. | Default: `"ce-amm"` | `^[a-z0-9._-]{3,64}$` | 3-64 |
| `service.version` | Versão determinística do serviço. | Derivada (Seção 4) | `^[A-Za-z0-9+._-]{2,64}$` | 2-64 |
| `deployment.environment` | Ambiente de execução. | Default: `dev` | `dev|stg|prod` (case-insensitive) | 3 |

## 3. Contexto adicional (não Resource)
| Campo | Descrição | Exemplo |
| ----- | --------- | ------- |
| `build.time.utc` | Instante exato do build, em UTC (RFC3339 estrito). | `2025-10-03T12:34:56Z` |
| `git.sha` | SHA completo (40 hex minúsculos) do commit. | `4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f4fd0c2a6` |

Os campos extras são expostos para logs/diagnóstico e **não** são propagados como atributos de Resource OTel.

## 4. Política de versionamento (`service.version`)
1. Se `SERVICE_VERSION` estiver definido e válido → utilizar literalmente.
2. Caso contrário, compor determinísticamente:
   - Se `CARGO_PKG_VERSION` **e** `CE_GIT_SHA_SHORT` (7 primeiros caracteres do hash) estiverem presentes → `"<CARGO_PKG_VERSION>+<CE_GIT_SHA_SHORT>"` (ex.: `2.1.0+1a2b3c4`).
   - Se apenas `CE_GIT_SHA_SHORT` estiver presente → `"0.0.0+<CE_GIT_SHA_SHORT>"`.
3. É **proibido** produzir rótulos genéricos como `unknown`, `dev`, `local`, ou strings sem hash.
4. Falhe explicitamente (`IdentityError::MissingVersion`) se nenhuma estratégia acima conseguir gerar uma versão válida.

## 5. Precedência de fontes
1. **Builder programático** – valores explicitamente setados via `ServiceIdentityBuilder`.
2. **Variáveis de ambiente** – `SERVICE_NAME`, `SERVICE_VERSION`, `DEPLOY_ENV`.
3. **Defaults** – `service.name = "ce-amm"`, `deployment.environment = dev`, `service.version` composto conforme Seção 4 com `CE_GIT_SHA`/`CE_GIT_SHA_SHORT` e `CARGO_PKG_VERSION`.

Os artefatos injetados via `build.rs` (`CE_BUILD_TIME_RFC3339`, `CE_GIT_SHA`, `CE_GIT_SHA_SHORT`) são considerados parte dos defaults obrigatórios.

## 6. Regras de validação
- `service.name` deve obedecer ao regex `^[a-z0-9._-]{3,64}$`; qualquer desvio gera `IdentityError::InvalidServiceName` com mensagem acionável.
- `service.version` deve satisfazer `^[A-Za-z0-9+._-]{2,64}$` e seguir a política da Seção 4.
- `deployment.environment` aceita apenas `dev`, `stg`, `prod` (case-insensitive), serializando em minúsculo.
- `build.time.utc` precisa estar em UTC (terminar com `Z`) e ser um timestamp RFC3339 válido.
- `git.sha` precisa ter 40 caracteres hexadecimais minúsculos; o curto (`CE_GIT_SHA_SHORT`) usa 7 caracteres.

## 7. Exemplos normativos
### 7.1 DEV com composição por git
```bash
export SERVICE_NAME=ce-amm
export DEPLOY_ENV=dev
# SERVICE_VERSION ausente; build.rs injeta CE_GIT_SHA=4fd0c2a64b7f1a3e9c0b2e1d5a6c7b8f4fd0c2a6 e CE_BUILD_TIME_RFC3339
# Resultado esperado:
service.name = "ce-amm"
service.version = "0.0.0+4fd0c2a"
deployment.environment = "dev"
```

### 7.2 STG com semver explícito
```bash
export SERVICE_NAME=ce-amm
export SERVICE_VERSION=2.3.1
export DEPLOY_ENV=stg
# Resultado esperado:
service.name = "ce-amm"
service.version = "2.3.1"
deployment.environment = "stg"
```

### 7.3 PROD com semver + git curto
```bash
export SERVICE_NAME=ce-amm
export DEPLOY_ENV=prod
# build.rs injeta CARGO_PKG_VERSION=2.4.0 e CE_GIT_SHA_SHORT=1a2b3c4
# Resultado esperado:
service.name = "ce-amm"
service.version = "2.4.0+1a2b3c4"
deployment.environment = "prod"
```

### 7.4 Casos inválidos (devem falhar)
- `SERVICE_NAME="CE AMM"` → caracteres inválidos (espaço/maiúsculas).
- `DEPLOY_ENV=production` → ambiente fora do domínio permitido (`dev|stg|prod`).
- `SERVICE_VERSION=dev-local` → não atende à regex/política (faltando hash).
- Ausência total de `SERVICE_VERSION` e impossibilidade de obter `CE_GIT_SHA` → falha de build (veja Seção 8).

## 8. Build e injeção de metadados
O `build.rs` garante:
- `CE_BUILD_TIME_RFC3339` com timestamp UTC (`Z`).
- `CE_GIT_SHA` (40 hex) e `CE_GIT_SHA_SHORT` (7 hex) derivados do commit atual.
- Falha com mensagem acionável se nenhuma fonte de hash estiver disponível (ex.: instrução para definir `GIT_COMMIT` no CI).

## 9. Consumo pelos módulos OBS-1
- **T4/T5/T6/T7** devem instanciar `ServiceIdentity` e usar `resource_pairs()` ao construir `Resource` de traces/métricas.
- `build.time.utc` e `git.sha` permanecem acessíveis em `ServiceIdentity` para logs estruturados e auditoria.
- Nenhum módulo OBS-1 deve redefinir chaves adicionais de Resource para identidade.

## 10. FAQ
**Q:** Preciso inicializar tracer/meter/log aqui?
**A:** Não. Este módulo apenas resolve identidade. Integrações OTel acontecem nas threads subsequentes (T4–T7).

**Q:** Como garantir o hash em ambientes sem git?
**A:** Configure `GIT_COMMIT` no pipeline antes do build. O script falha explicitamente se não conseguir resolver o hash.

**Q:** Posso usar outro nome de serviço?
**A:** Sim, desde que siga o regex e seja definido pelo builder ou `SERVICE_NAME` antes do build/deploy.
