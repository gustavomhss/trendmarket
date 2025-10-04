# OBS-1 Pós-Sync Runbook — versão 2025-10-04

## Objetivo
- Garantir que o fluxo pós-sync do pack OBS-1 valide código, telemetria e instrumentação antes de liberar mudanças para observabilidade.
- Reunir comandos obrigatórios, artefatos esperados e ações corretivas rápidas para manter métricas, traces e dashboards íntegros.

## Passo a passo canônico
1. `cargo check --all-targets`
   - Confirma que todos os binários e testes compilam com e sem instrumentação.
   - Se o pack exigir telemetria, reexecute com `cargo check --all-targets --features obs`.
2. `cargo test --all-targets`
   - Executa cobertura mínima dos cenários sem feature gating.
   - Para validar observabilidade, inclua `--features obs` ou defina `CARGO_FEATURES=obs`.
3. `make obs.test`
   - Encapsula suites específicas de métricas, traces e logs para OBS-1.
   - Armazena logs adicionais em `out/diagnostics/*.txt` quando ocorrerem falhas.
4. `./scripts/obs1_postsync_validate.sh`
   - Orquestra validações incrementais, rerodando testes condicionais com `--features obs` quando `cfg(feature = "obs")` for detectado.
   - Consolida a saída em `out/diagnostics/test-run-sync.txt`, `out/diagnostics/cfg-obs-uses.txt` e relatórios auxiliares.

> **Dica:** Ao trabalhar localmente, exporte `CARGO_FEATURES=obs` ou adicione `--features obs` aos comandos que precisam da instrumentação ativa. Sempre limpe artefatos antigos com `rm -f out/diagnostics/*.txt` antes de uma rodada final.

## Troubleshooting
- **Falha de compilação:** verificar `out/diagnostics/test-run-sync.txt` para a primeira ocorrência e aplicar `cargo clean` se a mensagem apontar para build cache.
- **Testes flakey de trace/métrica:** reexecutar `make obs.test` com `RUST_LOG=debug` e confirmar spans em `out/logs/*`.
- **Prometheus scrape vazio:** conferir `out/diagnostics/summary.md` e o endpoint local `http://localhost:9464/metrics` com `curl` para validar exposição.
- **Script abortado:** habilitar `set -x` temporariamente em `scripts/obs1_postsync_validate.sh` e inspecionar `out/diagnostics/*.txt` por stacktrace.

## Checklist rápido
- [ ] Métricas `obs1_metrics_prom` exportando séries críticas e scrape OK em Prometheus.
- [ ] Traces OTLP (`obs1_trace_contract.md`) com spans preenchidos e atributos `trace_id` propagados.
- [ ] Logs estruturados (`obs1_logs_contract.md`) sincronizados com os testes do script pós-sync.
- [ ] Dashboards/alertas revisados após atualizar artefatos em `out/diagnostics/`.
- [ ] Evidências anexadas aos watchers/metrics antes de solicitar revisão.
