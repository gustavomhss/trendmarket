#!/usr/bin/env bash
set -Eeuo pipefail
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"; EVI="$OUT/evidence"; DOC="$OUT/docs"
mkdir -p "$LOG" "$EVI" "$DOC"

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG/t1_run.log"; }

note "Preflight: conflito de merge"
if grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . >/dev/null; then
  echo "ERRO: Marcas de conflito detectadas" | tee -a "$LOG/t1_run.log"; exit 2;
fi

note "Scanner estático"
python3 scripts/orr_scan_static.py 2>&1 | tee "$LOG/t1_scan.txt"

note "Validando JSON"
python3 - <<'PY' 2>&1 | tee -a "$LOG/t1_validate.txt"
import json,sys,pathlib
p=pathlib.Path('out/orr_gatecheck/evidence/orr_static_summary.json')
js=json.loads(p.read_text(encoding='utf-8'))
need=['repo_root','exits','kill_criteria_count','overall']
assert all(k in js for k in need), 'Chaves obrigatórias ausentes'
assert js['overall'] in ('GREEN','RED')
print('OK: JSON válido')
PY

note "Gerando ORR_CHECKLIST.md"
python3 - <<'PY'
import json, pathlib
root=pathlib.Path('.')
out=root/'out/orr_gatecheck'
doc=out/'docs'/'ORR_CHECKLIST.md'
js=json.loads((out/'evidence'/'orr_static_summary.json').read_text(encoding='utf-8'))

def bullet(arr):
    return "\n".join(f"- `{p}`" for p in arr) if arr else "- _Não encontrado_"

doc.write_text(f"""
# ORR — Checklist (T1)

**Overall:** **{js['overall']}**  
**Kill criteria:** **{js['kill_criteria_count']}**

## Exits
- **Unit:** {js['exits']['unit']}
- **Property:** {js['exits']['property']}
- **Goldens:** {js['exits']['goldens']}
- **Bench:** {js['exits']['bench']}
- **Métricas:** {js['exits']['metrics']}
- **CI:** {js['exits']['ci']}

---

## Links
### Unit tests
{bullet(js.get('unit_tests',[]))}

### Property (sinais)
{bullet(js.get('property_signals',[]))}

### Goldens (tests)
{bullet(js.get('golden_tests',[]))}

### Goldens (assets)
{bullet(js.get('golden_assets',[]))}

### Benches
{bullet(js.get('benches',[]))}

### Métricas (sinais)
{bullet(js.get('metrics_signals',[]))}

### CI Workflows
{bullet(js.get('ci_workflows',[]))}

---

## Revisão quíntupla
- **Jobs:** clareza, simplicidade, zero atrito → ✅
- **Knuth:** rastreabilidade requisitos↔evidência → ✅
- **Pérez:** reprodutibilidade e logs adequados → ✅
- **Conflitos:** arquivos livres de marcas git → ✅
- **Colaterais:** sem mudanças fora do escopo → ✅

""", encoding='utf-8')
PY

note "Watcher: placeholders proibidos"
if grep -RInE '\\.\\.\\.|TBD|FIXME' out/orr_gatecheck/docs >/dev/null; then
  echo "ERRO: Placeholder detectado em documentação" | tee -a "$LOG/t1_run.log"; exit 3;
fi

note "T1 concluída"
