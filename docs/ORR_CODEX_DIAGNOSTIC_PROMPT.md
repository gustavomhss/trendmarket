# Codex Diagnostic Prompt (Read-Only Runner)

```
META
- Repo canônico: git@github.com:gustavomhss/trendmarket.git (branch main).
- Ambiente: possivelmente read-only; sem shell/python/jq/gh.
- Objetivo: preparar PR que permita execução local (macOS) para fechar ORR GREEN real, mantendo T8 estrito (sem greenwashing) e T7 sem fallback fantasia.

TAREFAS
1) Crie/edite apenas os arquivos em scripts/orr_t{1..8}_*.(sh|py) + scripts/orr_env_probe.sh + docs/ORR_OFFLINE.md + (opcional) scripts/orr_dryrun_fixtures/*.
2) Em cada runner, implemente detecção de read-only: se não puder escrever/rodar, encerre com exit code 95 e IMPRIMA no STDOUT um JSON de diagnóstico minimalista (um por linha), sem gravar nada no disco.
3) Garanta atomicidade de escrita (tmp + mv) e `mkdir -p` antecipado.
4) Remova qualquer uso de recursos não portáveis a bash 3.2 (ex.: mapfile, process substitution complexa). Sem subshells extravagantes.
5) T5: o coletor deve sair com **exit 3** quando `count==0` para acionar o fallback em orquestração.
6) T7: só escreva `run_summary.json` quando `gh`+rede estiverem OK; caso contrário, STDOUT com diagnóstico e exit 2. Calcule `duration_seconds` a partir de `startedAt`/`updatedAt`.
7) T8: use `rd_safe()` para tolerar ausências/corrupção, marcando `RED` no respectivo exit. **overall=GREEN** apenas quando todos forem `GREEN`.
8) Forneça no PR um documento `docs/ORR_OFFLINE.md` com os códigos de saída e amostras de STDOUT para RO/sem `gh`.
9) Saída final do seu trabalho: imprima um único JSON com `{"changed_files":[...],"lint":"ok","macos_portability":"ok","notes":[...]}`.

VALIDAÇÃO ESTÁTICA (sem executar)
- Verifique sintaxe bash (bash -n mental), POSIX compat, e Python 3.8+ compat.
- Cheque que todos os caminhos `out/orr_gatecheck/**` são criados via `mkdir -p` antes da escrita.
- Garanta que nenhuma mudança toca `src/**` de forma persistente.
- Use comentários mínimos no código; o detalhamento vai para docs/ORR_OFFLINE.md.

CRITÉRIOS DE ACEITE
- Nenhum placeholder.
- Scripts compatíveis com bash 3.2.
- T8 estrito (sem marcar GREEN na ausência de evidências) e resiliente.
- T7 sem fallback “fake”: se offline, não cria arquivo e retorna exit 2.
- PR único, limpo, com changelog e exemplos de STDOUT para RO.
```

