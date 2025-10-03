# OBS-1 Política de Lints para PII e Cardinalidade

Este documento normativo consolida as regras contratuais do pack **CRD-8 / OBS-1** para logs e métricas.
Ele define os campos permitidos, os campos proibidos e os mecanismos de validação que todo serviço deve
utilizar antes de enviar dados para as camadas de observabilidade.

## 1. Objetivos

- **Bloquear PII** antes de qualquer emissão para `stderr`, OTLP ou destinos secundários.
- **Congelar cardinalidade** de métricas a apenas quatro labels estáveis: `op`, `service`, `env`, `version`.
- **Fornecer APIs determinísticas** (`obs_policy_lints`) que possam ser usadas tanto em build quanto em runtime.
- **Garantir evidências auditáveis** das verificações.

## 2. Campos Permitidos e Proibidos

### 2.1 Labels de métricas

- Permitidos: `op`, `service`, `env`, `version`.
- Proibidos (regex literal): `(?i)(user_id|account_id|request_id|session_id|.*_uuid|.*_hash)`.
- Qualquer chave fora da lista canônica gera `PolicyError::ForbiddenLabel` (falha dura).

### 2.2 Logs JSON

- Campos proibidos (raiz ou `extra`): `email`, `cpf`, `phone`, `address`, `name`, `geo`, `person_*`.
- Regex literal aplicada em chaves: `(?i)^(email|cpf|phone|address|name|geo|person_.*)$`.
- A ação padrão é rejeitar o evento (`ScrubMode::Reject`). Quando configurado em `ScrubMode::Redact`, o
  campo aparece como `"[redacted]"`.

### 2.3 Exemplos normativos

#### Log com PII (rejeitado)

```json
{"ts":"2025-10-03T12:34:56Z","level":"info","msg":"swap","service":"ce-amm","env":"dev","version":"2.4.0+1a2b3c4","email":"cliente@exemplo.com"}
```

> Resultado esperado: erro `PiiDetected("email")`.

#### Labels válidos

```text
[("op","swap"),("service","ce-amm"),("env","dev"),("version","2.4.0+1a2b3c4")]
```

#### Label proibido (reprovado)

```text
[("op","swap"),("request_id","abcd"),("env","dev"),("version","2.4.0+1a2b3c4")]
```

> Resultado: `ForbiddenLabel("request_id")`.

## 3. APIs do módulo `obs_policy_lints`

### 3.1 `validate_metric_labels`

```rust
use credit_engine_core::obs_policy_lints::{validate_metric_labels, PolicyError};

let labels = vec![
    ("op", "swap"),
    ("service", "ce-amm"),
    ("env", "dev"),
    ("version", "2.4.0+1a2b3c4"),
];
validate_metric_labels(&labels)?; // retorna Err(PolicyError::ForbiddenLabel) quando inválido
```

### 3.2 `contains_pii_key`

```rust
use credit_engine_core::obs_policy_lints::contains_pii_key;
use serde_json::json;

let candidate = json!({
    "msg": "swap",
    "extra": {"phone": "+5511988887777"}
});
assert!(contains_pii_key(candidate.as_object().unwrap()));
```

### 3.3 `scrub_log`

```rust
use credit_engine_core::obs_policy_lints::{scrub_log, ScrubMode};
use serde_json::json;

let log = json!({
    "msg": "swap",
    "email": "cliente@exemplo.com"
});
let result = scrub_log(log, ScrubMode::Reject); // Err(PolicyError::PiiDetected("email"))
```

Quando configurado em `ScrubMode::Redact`, o campo proibido é substituído por `"[redacted]"` tanto na raiz quanto
em `extra`.

### 3.4 `PiiGuardLayer`

O layer implementa `tracing_subscriber::Layer` e deve ser injetado **antes** da layer JSON (T7) na
composição do subscriber. Em `ScrubMode::Reject` ele **dropa** o evento; em `ScrubMode::Redact` os campos
proibidos chegam ao formatter como `"[redacted]"`.

```rust
use credit_engine_core::obs_policy_lints::{PiiGuardLayer, ScrubMode};
use credit_engine_core::telemetry_logs::{json_layer, LogConfig};
use tracing_subscriber::{layer::SubscriberExt, Registry};

let guard = PiiGuardLayer::new(ScrubMode::Reject);
let json = json_layer(&LogConfig {
    level: "info".into(),
    service: "ce-amm".into(),
    env: "prod".into(),
    version: "2.4.0+1a2b3c4".into(),
})?;
let subscriber = Registry::default().with(json).with(guard);
tracing::subscriber::set_global_default(subscriber)?;
```

## 4. Integração Recomendada

1. **Logging**: adicione `PiiGuardLayer` antes da layer JSON configurada em T7. Em pipelines existentes que usam
   `json_layer`, basta encadear `.with(PiiGuardLayer::new(ScrubMode::Reject))` por último para que o guard fique na borda.
2. **Métricas**: antes de qualquer `Counter::add` ou `Histogram::record`, passe os labels por `validate_metric_labels`.
   Mantenha um helper local (ex.: `fn safe_labels(...)`) que chama o validador.
3. **Sanitização ad-hoc**: para payloads JSON, utilize `scrub_log(value, mode)` antes de encaminhar para filas ou storage.
4. **CI / Hooks**: execute `scripts/obs_policy_scan.sh` em CI, e opcionalmente como pre-commit, para garantir que nenhum
   arquivo novo introduziu PII ou labels fora da política.

## 5. Scanner de Repositório

- Caminho: `scripts/obs_policy_scan.sh`.
- Escopo: `src/**`, `docs/**`, `schemas/**`, `*.yaml` na raiz.
- Saída: `out/obs_gatecheck/evidence/obs_policy_scan.json` com contagens e trechos.
- Retorno: `0` quando não existem ocorrências; `>0` quando há violações não listadas em `.obs_policy_allowlist`.
- O arquivo `.obs_policy_allowlist` (opcional) aceita entradas `path:substring` para tolerar falsos positivos controlados.

## 6. FAQ

- **Posso logar PII em ambientes de desenvolvimento?** Não. O contrato T1 é universal e as camadas bloquearão a emissão.
- **Preciso redigir ou rejeitar?** O modo padrão é `Reject`. Só habilite `Redact` quando existir justificativa explícita
  e auditoria da área de privacidade.
- **Como auditar?** Combine os relatórios `obs_policy_lints_report.json` e `obs_policy_scan.json` com os hashes de build.

## 7. Checklist de Adoção

- [ ] `PiiGuardLayer` encadeado antes da layer JSON.
- [ ] `validate_metric_labels` aplicado a toda gravação de métrica.
- [ ] `scripts/obs_policy_scan.sh` adicionado ao pipeline de CI (e opcionalmente ao pre-commit).
- [ ] Evidências publicadas em `out/obs_gatecheck/evidence/`.
