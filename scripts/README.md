# Scripts

## Hash & Metadados (Thread 6)

O script `obs3_hash_manifest.py` executa a etapa de hashing e enriquecimento de metadados do manifesto de evidências do OBS-3. Execute-o após a geração das evidências (Thread 4) e opcionalmente depois dos quality checks (Thread 5), garantindo que os arquivos JSON estejam consolidados em `out/obs_gatecheck/evidence/`.

### Campos gerados
- `run_id`: UUID4 único a cada execução.
- `spec_version`: versão do manifesto (default `5.0`, configurável via flag).
- `git_sha`: commit usado para produzir as evidências (`git rev-parse HEAD` ou valor passado).
- `ts`: timestamp UTC ISO8601 (`...Z`).
- `integrity`: mapa `arquivo.json → sha256` dos artefatos de evidência.

Somente arquivos `.json` diretamente no diretório de evidências são considerados, excluindo o próprio manifesto (`prom_scrape.json` por padrão). Subdiretórios são ignorados para manter determinismo.

### Uso
```bash
python3 scripts/obs3_hash_manifest.py \
  --evidence-dir out/obs_gatecheck/evidence \
  --manifest out/obs_gatecheck/evidence/prom_scrape.json \
  --spec-version 5.0 --pretty
```
Flags úteis:
- `--git-sha`: sobrescreve o SHA detectado.
- `--dry-run`: imprime o JSON resultante sem gravar.
- `--verbose`: log detalhado no stderr.

### Exit codes e troubleshooting
- `0`: manifesto escrito (ou impresso) com sucesso.
- `5`: nenhum arquivo de evidência `.json` elegível encontrado — confirme a saída da Thread 4.
- `13`: erro de E/S (permissão, escrita, rename).
- `14`: JSON inválido ao ler manifesto existente — corrija o arquivo ou remova-o.
- `15`: falha ao calcular `sha256` (arquivo inacessível/corrompido).

O script realiza escrita atômica (`*.tmp` + rename) e falha de forma explícita sempre que encontra problemas. Para reconstruir o manifesto, garanta que as evidências estejam acessíveis e que o diretório de destino seja gravável.
