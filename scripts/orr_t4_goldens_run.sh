#!/usr/bin/env bash
set -Eeuo pipefail
export LC_ALL=C
ROOT="$(pwd)"
OUT="$ROOT/out/orr_gatecheck"
LOG="$OUT/logs"; EVI="$OUT/evidence/goldens"; DIFF="$EVI/diff_reports"; DOC="$OUT/docs"
mkdir -p "$LOG" "$EVI" "$DIFF" "$DOC"

note(){ printf "[%s] %s\n" "$(date +%FT%T%z)" "$*" | tee -a "$LOG/t4_run.log"; }

note "Higiene: conflitos e placeholders"
if grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . >/dev/null; then echo "ERRO: Conflitos detectados" | tee -a "$LOG/t4_run.log"; exit 2; fi
if grep -RInE '\\.\\.\\.|TBD|FIXME' -n tests/goldens goldens >/dev/null 2>&1; then echo "ERRO: Placeholder detectado" | tee -a "$LOG/t4_run.log"; exit 3; fi

note "Executando suíte golden"
mapfile -t GOLDEN_SOURCES < <(find tests -maxdepth 1 -type f -name 'golden_*.rs' | sort)
if [ "${#GOLDEN_SOURCES[@]}" -eq 0 ]; then
  echo "ERRO: Nenhum teste golden encontrado" | tee -a "$LOG/t4_run.log"
  exit 7
fi
GOLDEN_BINARIES=()
for src in "${GOLDEN_SOURCES[@]}"; do
  file_name="$(basename "$src")"
  GOLDEN_BINARIES+=("${file_name%.rs}")
done
: > "$LOG/cargo_test_goldens.txt"
GOLDEN_FAILURE=0
for test_bin in "${GOLDEN_BINARIES[@]}"; do
  note "cargo test --test ${test_bin} -- --nocapture"
  if ! cargo test --test "${test_bin}" -- --nocapture 2>&1 | tee -a "$LOG/cargo_test_goldens.txt"; then
    GOLDEN_FAILURE=1
  fi
done

note "Coletando expected vs actual"
# Convention: testes escrevem saídas atuais sob out/orr_gatecheck/evidence/goldens/actual/
# Se os testes ainda não escrevem, adicione redirecionos neles. Nesta thread, assumimos que já existem goldens em tests/goldens/* e fixtures em goldens/*.
mkdir -p "$EVI/actual" "$EVI/expected"
# Copiar fixtures expected
if [ -d goldens ]; then
  rsync -a --delete goldens/ "$EVI/expected/"
fi
# Caso alguns testes já produzam arquivos atuais em paths previsíveis, eles devem estar em $EVI/actual/

note "Hashes (expected & actual)"
python3 - <<'PY'
import hashlib, json, pathlib
E=pathlib.Path('out/orr_gatecheck/evidence/goldens/expected')
A=pathlib.Path('out/orr_gatecheck/evidence/goldens/actual')

def sha(p: pathlib.Path):
    h=hashlib.sha256(); h.update(p.read_bytes()); return h.hexdigest()

def scan(base):
    out=[]
    if not base.exists(): return out
    for p in sorted([x for x in base.rglob('*') if x.is_file()]):
        out.append({"path": str(p.relative_to(base)), "sha256": sha(p), "size": p.stat().st_size})
    return out

exp=scan(E); act=scan(A)
(pathlib.Path('out/orr_gatecheck/evidence/goldens/hashes_expected.json')).write_text(json.dumps(exp,indent=2), encoding='utf-8')
(pathlib.Path('out/orr_gatecheck/evidence/goldens/hashes_actual.json')).write_text(json.dumps(act,indent=2), encoding='utf-8')
PY

note "Diffs (expected vs actual)"
python3 - <<'PY'
import json, pathlib, subprocess, sys, shutil
ROOT=pathlib.Path('.')
EVI=ROOT/'out/orr_gatecheck/evidence/goldens'
EXP=EVI/'expected'
ACT=EVI/'actual'
DIFF=EVI/'diff_reports'
if DIFF.exists():
    shutil.rmtree(DIFF)
DIFF.mkdir(parents=True, exist_ok=True)

actual_files = sorted([p for p in ACT.rglob('*') if p.is_file()])
expected_files = sorted([p for p in EXP.rglob('*') if p.is_file()])
all_rels = sorted({p.relative_to(ACT) for p in actual_files} | {p.relative_to(EXP) for p in expected_files})

compared=0
mismatch=0
for rel in all_rels:
    ep = EXP/rel
    ap = ACT/rel
    if ep.exists() and ap.exists():
        compared += 1
        try:
            out = subprocess.run(
                ['diff', '-u', '--label', f"expected/{rel}", '--label', f"actual/{rel}", str(ep), str(ap)],
                capture_output=True,
                text=True,
            )
        except FileNotFoundError:
            mismatch += 1
            diff_file = DIFF/f"{rel}.diff"
            diff_file.parent.mkdir(parents=True, exist_ok=True)
            diff_file.write_text('[DIFF TOOL UNAVAILABLE]\n', encoding='utf-8')
        else:
            if out.returncode != 0:
                mismatch += 1
                diff_file = DIFF/f"{rel}.diff"
                diff_file.parent.mkdir(parents=True, exist_ok=True)
                diff_file.write_text(out.stdout or '[BINARY OR NO DIFF OUTPUT]\n', encoding='utf-8')
    elif ep.exists() and not ap.exists():
        mismatch += 1
        diff_file = DIFF/f"{rel}.diff"
        diff_file.parent.mkdir(parents=True, exist_ok=True)
        diff_file.write_text('[MISSING ACTUAL]\n', encoding='utf-8')
    elif ap.exists() and not ep.exists():
        mismatch += 1
        diff_file = DIFF/f"{rel}.diff"
        diff_file.parent.mkdir(parents=True, exist_ok=True)
        diff_file.write_text('[MISSING EXPECTED]\n', encoding='utf-8')

summary = {
    'expected_files': len(expected_files),
    'actual_files': len(actual_files),
    'compared': compared,
    'mismatch': mismatch,
    'status': 'GREEN' if mismatch == 0 else 'RED',
}
(EVI/'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')
print(json.dumps(summary, indent=2))
if mismatch != 0:
    sys.exit(8)
PY

if [ "$GOLDEN_FAILURE" -ne 0 ]; then
  echo "ERRO: Falha na suíte golden (ver out/orr_gatecheck/logs/cargo_test_goldens.txt)" | tee -a "$LOG/t4_run.log"
  exit 9
fi

note "Watcher: finais"
! grep -RInE '\\.\\.\\.|TBD|FIXME' out/orr_gatecheck/docs || { echo "ERRO: placeholder na doc"; exit 4; }
! grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . || { echo "ERRO: conflitos no repo"; exit 5; }

note "Atualização controlada (opcional)"
if [ "${UPDATE_GOLDENS:-0}" = "1" ]; then
  echo "UPDATE_GOLDENS=1 → sincronizando actual → goldens/ (controle estrito)" | tee -a "$LOG/t4_run.log"
  rsync -a --delete "$EVI/actual/" goldens/
  echo "ATENÇÃO: revise o diff antes do commit." | tee -a "$LOG/t4_run.log"
fi

note "T4 concluída"
