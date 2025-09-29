# Triple Review Checklist — SUB-08

## Lógica
- [x] Símbolos das fórmulas cobrem a notação (validação automática sem faltas).
- [x] IDs de rounding referenciados nos exemplos (RX1–RX5 + notas complementares) batem com `out/docs/rounding_matrix.csv`.
- [x] Seções obrigatórias presentes: Notação, Fórmulas do Módulo, Política de Rounding, Build/Test/Bench, ADRs & CI.

## Sintaxe
- [x] Markdown validado visualmente (tabelas e listas fechadas, code blocks com linguagem `text` ou `rust`).
- [x] JSON/CSV auxiliares permanecem inalterados (somente leitura/validação).
- [x] Ausência de placeholders (`...`, `TBD`, `FIXME`).

## Identação & Estilo
- [x] Segue estilo existente (português técnico, listas com hifens, headings `##`).
- [x] Backticks apenas para código/paths relevantes; nomes de funções em notas sem inline code para evitar validação falsa.
- [x] Novas seções respeitam formatação do README (parágrafos curtos, bullet lists, referências cruzadas).
