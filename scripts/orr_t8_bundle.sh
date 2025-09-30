#!/usr/bin/env bash
set -Eeuo pipefail
export LC_ALL=C
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"; EVI="$OUT/evidence"; DOC="$OUT/docs"
mkdir -p "$LOG" "$EVI" "$DOC"

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG/t8_run.log"; }

note "Higiene: conflitos e placeholders"
if grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . >/dev/null; then echo "ERRO: Conflitos detectados" | tee -a "$LOG/t8_run.log"; exit 2; fi
if grep -RInE '\.\.\.|TBD|FIXME' -n out/orr_gatecheck docs .github/workflows 2>/dev/null; then echo "ERRO: Placeholder detectado" | tee -a "$LOG/t8_run.log"; exit 3; fi

note "Agregando e validando evidências"
python3 scripts/orr_t8_validate.py 2>&1 | tee "$LOG/t8_validate.txt" || true

FINAL_JSON="$EVI/orr_final_summary.json"
[ -f "$FINAL_JSON" ] || { echo "ERRO: resumo final ausente" | tee -a "$LOG/t8_run.log"; exit 4; }
OVERALL=$(jq -r '.overall' "$FINAL_JSON" 2>/dev/null || echo RED)
KILL=$(jq -r '.kill_criteria_count' "$FINAL_JSON" 2>/dev/null || echo 99)

note "Escrevendo ORR_README.md"
python3 - <<'PY'
import json, pathlib
base=pathlib.Path('out/orr_gatecheck')
js=json.loads((base/'evidence/orr_final_summary.json').read_text())
rd=base/'docs'/'ORR_README.md'

def bl(links):
    s=[]
    for p in links:
        s.append(f"- `{p}`")
    return "\n".join(s)

rd.write_text(f"""
# ORR — Bundle Final (T8)

## Resultado
- **Overall:** **{js['overall']}**
- **Kill criteria:** **{js['kill_criteria_count']}**

## Checklist com links
### Unit
{bl(js['checklist_links']['unit'])}

### Property
{bl(js['checklist_links']['property'])}

### Goldens
{bl(js['checklist_links']['goldens'])}

### Bench
{bl(js['checklist_links']['bench'])}

### Métricas
{bl(js['checklist_links']['metrics'])}

### CI
{bl(js['checklist_links']['ci'])}

## Como usar
1. Abra os arquivos em `out/orr_gatecheck/docs/*` para leitura humana.
2. Use os JSONs em `out/orr_gatecheck/evidence/*` como fonte de verdade para gates automatizados.
3. Anexe o ZIP gerado por esta thread ao PR.
""", encoding='utf-8')
PY

note "Escrevendo ORR_APPROVAL.md (preenchimento automático)"
python3 - <<'PY'
import json, pathlib, datetime
base=pathlib.Path('out/orr_gatecheck')
js=json.loads((base/'evidence/orr_final_summary.json').read_text())
md=base/'docs'/'ORR_APPROVAL.md'
app = 'Aprovado ✅' if js['overall']=='GREEN' and js['kill_criteria_count']==0 else 'Reprovado ❌'
md.write_text(f"""
# ORR — Aprovação (T8)

**Decisão:** {app}  
**Responsável:** ___________________________  
**Data/Hora:** {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}

## Exits
- Unit: {js['exits']['unit']}
- Property: {js['exits']['property']}
- Goldens: {js['exits']['goldens']}
- Bench: {js['exits']['bench']}
- Métricas: {js['exits']['metrics']}
- CI: {js['exits']['ci']}

## Revisão quíntupla
- **Jobs:** clareza e acabamento → ☐
- **Knuth:** rastreabilidade e narrativa → ☐
- **Pérez:** reprodutibilidade e logs → ☐
- **Conflitos:** repo limpo → ☐
- **Colaterais:** escopo respeitado → ☐
""", encoding='utf-8')
PY

note "Gerando ZIP final"
STAMP=$(date +%Y%m%d-%H%M%S)
BZIP="out/orr_gatecheck_bundle-$STAMP.zip"
(
  cd out && zip -qr "orr_gatecheck_bundle-$STAMP.zip" orr_gatecheck
) || { echo "ERRO: falha ao gerar ZIP (instale 'zip')" | tee -a "$LOG/t8_run.log"; exit 5; }

echo "BUNDLE=$BZIP" | tee "$OUT/STATUS_T8.env"

if [ "$OVERALL" != "GREEN" ] || [ "$KILL" != "0" ]; then
  echo "ERRO: ORR não GREEN (OVERALL=$OVERALL, KILL=$KILL). Corrija threads pendentes e reexecute." | tee -a "$LOG/t8_run.log"
  exit 6
fi

note "T8 concluída — bundle pronto"
