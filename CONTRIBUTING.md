# Contribuindo para o CreditEngine OBS-3

Obrigado por contribuir! Siga estas diretrizes para manter o padrão auditável:

1. **Planejamento**
   - Abra um RFC utilizando `docs/RFC_TEMPLATE.md` para mudanças estruturais.
   - Sincronize com os owners definidos em `.github/CODEOWNERS`.

2. **Desenvolvimento**
   - Use `python -m venv` ou ferramentas equivalentes para isolar dependências.
   - Execute `pre-commit install` para habilitar os hooks locais.
   - Rode `make pr-check` antes de abrir o PR.

3. **Observabilidade**
   - Atualize `ops/prometheus` com novos alvos ou regras quando necessário.
   - Garanta que `promtool test rules` cubra o cenário nominal e as caudas.
   - Gere evidências via `./scripts/obs_t3_prom_scrape_run.sh` e anexe os artefatos.

4. **Governança**
   - PRs devem seguir o formato Conventional Commits (`feat:`, `fix:`, etc.).
   - Preencha o `RELATÓRIO DE EXECUÇÃO` na descrição do PR.
   - Solicite pelo menos duas revisões e assegure que as conversas foram resolvidas.

5. **Pós-merge**
   - Atualize a documentação (`docs/CHANGELOG.md`, `docs/QA_CHECKLIST.md`).
   - Verifique se o workflow `OBS-3 Prometheus CI` permaneceu verde após o merge.

## Ambiente local rápido

```bash
pip install -r requirements.txt
pre-commit install
make help
```

Para dúvidas, mencione `@gustavomhss` no PR.
