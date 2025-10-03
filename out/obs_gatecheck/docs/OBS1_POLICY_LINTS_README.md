# OBS-1 Policy Lints — Guia Operacional

Este README explica como utilizar a biblioteca `obs_policy_lints`, como executar o scanner e como interpretar
as evidências produzidas para o pack OBS-1.

## 1. Componentes

- **Biblioteca (`src/obs_policy_lints.rs`)**: expõe as constantes contratuais e as funções `validate_metric_labels`,
  `contains_pii_key`, `scrub_log` e o layer `PiiGuardLayer`.
- **Scanner (`scripts/obs_policy_scan.sh`)**: varre `src/**`, `docs/**`, `schemas/**` e arquivos `*.yaml` na raiz em busca
  de PII e labels proibidos.
- **Testes (`tests/obs_policy_lints_tests.rs`)**: validam os caminhos de sucesso e falha para logs, métricas e para o layer.
- **Evidências (`out/obs_gatecheck/evidence/*.json`)**: relatórios com timestamp, contagens, hashes e amostras.

## 2. Usando o `PiiGuardLayer`

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

- Use `ScrubMode::Reject` por padrão. `ScrubMode::Redact` mantém o evento, substituindo os campos proibidos por `"[redacted]"`.
- Posicione o guard como o último `.with(...)` para que ele seja avaliado antes das demais layers.

## 3. Validando métricas

```rust
use credit_engine_core::obs_policy_lints::validate_metric_labels;

let labels = vec![
    ("op", "swap"),
    ("service", "ce-amm"),
    ("env", "prod"),
    ("version", "2.4.0+1a2b3c4"),
];
validate_metric_labels(&labels)?; // qualquer chave inválida gera PolicyError::ForbiddenLabel
```

## 4. Executando o scanner

```bash
scripts/obs_policy_scan.sh
```

- Saída: `out/obs_gatecheck/evidence/obs_policy_scan.json`.
- Retorno: `0` quando limpo; `1` (ou maior) quando há ocorrências não listadas em `.obs_policy_allowlist`.
- Utilize `.obs_policy_allowlist` para tratar falsos positivos pontuais (formato `path:substring`).

### Exemplo de saída

```json
{
  "timestamp": "2025-01-01T12:00:00Z",
  "summary": {"pii": 0, "labels": 0, "total": 0},
  "matches": {"pii": [], "labels": []}
}
```

## 5. Evidências complementares

- `out/obs_gatecheck/evidence/obs_policy_lints_report.json`: inclui timestamp, resultado de testes e hashes SHA256 dos arquivos
  relevantes (`src/obs_policy_lints.rs`, script, testes, docs).
- `out/obs_gatecheck/evidence/obs_policy_scan.json`: produto direto do scanner.

Combine os dois relatórios na publicação de evidências para garantir rastreabilidade de testes e de conteúdo escaneado.

## 6. Checklist rápido

- [ ] Layer `PiiGuardLayer` configurado em produção.
- [ ] Labels de métricas validados antes da emissão.
- [ ] Scanner incluído no pipeline (CI e/ou pre-commit).
- [ ] Evidências atualizadas em `out/obs_gatecheck/evidence/`.
