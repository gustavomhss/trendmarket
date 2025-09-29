## Resumo
Consolidamos a montagem final do README com Notação, Fórmulas, Política de Rounding e exemplos didáticos/realistas sincronizados com os artefatos das threads A–F.
Também alinhamos Build, Test & Bench e ADRs & CI às validações finais e ao inventário consolidado.

## Escopo
- Apenas documentação e artefatos em `out/**`; nenhuma alteração de lógica.

## Evidências
- Validações: veja `out/docs/sweep_report.json` e `out/docs/validation_report.json`.
- Logs: `out/logs/**` (A–F, SUB‑08, SUB‑09).
- Arquivos criados/editados (SUB‑09): ver `out/logs/sub09/created_files_sub09.txt`, `edited_files_sub09.txt`.

## Como testar
- Siga *Build, Test & Bench* no README.
- Snippets dos exemplos executam e batem com os valores documentados.

## Checklist
- [x] Sem placeholders (`...`, `TBD`, `FIXME`).
- [x] Lint de Markdown sem erros críticos.
- [x] Links internos/externos válidos.
- [x] Exemplos (didáticos/realistas) verificam contra a implementação.
- [x] ADRs & CI com badges/links válidos.

## Riscos/Colaterais
Baixo — apenas documentação. Conferir anchors e TOC.

## Links úteis
- ADR Index: ver `README § ADRs & CI` e `out/docs/adr_index.json`.
- CI Inventory: `out/docs/ci_inventory.json`.
