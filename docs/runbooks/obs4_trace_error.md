# RB-TRACE-ERROR — Erros sem `StatusCode::ERROR`

## Sintomas
- Exceções conhecidas aparecem apenas como `StatusCode::UNSET` nas ferramentas de trace.
- Dashboards de erro em Tempo/Jaeger não refletem falhas reais da aplicação.

## Checagens
- Revisar uso de `set_status`/`set_status_code` no SDK: chamadas ausentes ou sobrescritas após captura da exceção.
- Confirmar mapeamento de exceções → status no middleware/integrações (ex.: `Tracer::record_exception`).
- Verificar políticas `status_code` no tail sampling: regras que descartam spans com `ERROR` podem mascarar o sinal.
- Validar export final (OTLP/Jaeger) com span real usando `otel-cli` ou `tempo-cli`, garantindo que `status.code` chega ao backend.

## Ações
1. Reproduzir o erro usando o demo bin (`scripts/obs/demo_error_span.sh`) para obter um span com falha controlada.
2. Instrumentar localmente com log/print do `set_status` confirmando que `StatusCode::ERROR` é aplicado antes do end span.
3. Ajustar o mapeamento de exceções ou garantir que o middleware de erro propague o status correto.
4. Revisar a política do tail sampler (`status_code` include/exclude) e remover filtros que descartam `ERROR`.
5. Validar export com `tempo-cli inspect <trace_id>` ou consulta equivalente e anexar evidências.

## Artefatos
- Captura do comando demo (trace_id, saída).
- Trecho de configuração do sampler mostrando ajuste aplicado.
- Export do span (JSON/OTLP) com `status.code = ERROR` confirmando correção.
