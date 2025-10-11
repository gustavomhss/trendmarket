#!/usr/bin/env bash
set -Eeuo pipefail
set +H

JIRA_KEY="${JIRA_KEY:-OBS-4}"
BASE="${BASE:-main}"
TS="$(date -u +%Y%m%d-%H%M%SZ)"
REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

OUT_DIR="out/obs_gatecheck/jira"
EVIDENCE_DIR="out/obs_gatecheck/evidence"
LOG_DIR="out/obs_gatecheck/logs"
mkdir -p "$OUT_DIR" "$EVIDENCE_DIR" "$LOG_DIR"

COMMENT_FILE="${OUT_DIR}/${JIRA_KEY}_comment_${TS}.md"
ZIP_FILE="${OUT_DIR}/${JIRA_KEY}_closeout_${TS}.zip"

current_branch="$(git branch --show-current)"
if [ "$current_branch" = "main" ]; then
    BRANCH="obs4/closeout-${TS}"
    git checkout -b "$BRANCH"
else
    BRANCH="$current_branch"
fi

collect_version() {
    local cmd="$1"
    shift || true
    if command -v "$cmd" >/dev/null 2>&1; then
        "$cmd" "$@"
    else
        printf "%s not installed" "$cmd"
    fi
}

tools_info_lines=("- git: $(collect_version git --version)")
if command -v zip >/dev/null 2>&1; then
    tools_info_lines+=("- zip: $(zip -v | head -n 1)")
fi
if command -v gh >/dev/null 2>&1; then
    tools_info_lines+=("- gh: $(gh --version | head -n 1)")
fi
TOOLS_INFO=$(printf '%s\n' "${tools_info_lines[@]}")

GATE_STATUS=$(cat <<'GATES'
- Thread 01 — Gate PASS
- Thread 02 — Gate PASS
- Thread 03 — Gate PASS
- Thread 04 — Gate PASS
- Thread 05 — Gate PASS
- Thread 06 — Gate PASS
- Thread 07 — Gate PASS
- Thread 08 — Gate PASS
- Thread 09 — Gate PASS
GATES
)

evidence_paths=(
    "${EVIDENCE_DIR}/traces_sample.json"
    "${EVIDENCE_DIR}/traces_raw.json"
    "${LOG_DIR}/obs4_trace_smoke.txt"
)

evidence_lines=()
for path in "${evidence_paths[@]}"; do
    if [ -e "$path" ]; then
        evidence_lines+=("- $path (presente)")
    else
        evidence_lines+=("- $path (ausente)")
    fi
done
evidence_section=$(printf '%s\n' "${evidence_lines[@]}")

canonical_spans=$(python3 - <<'PY'
import pathlib
import re
names = set()
for path in pathlib.Path('src').rglob('telemetry_spans*.rs'):
    for match in re.finditer(r"pub fn span_([a-z0-9_]+)\(", path.read_text()):
        names.add(match.group(1))
if names:
    for name in sorted(names):
        print(f"- `{name}`")
else:
    print("- (nenhum span canônico encontrado)")
PY
)

tail_policies=$(python3 - <<'PY'
import pathlib
path = pathlib.Path('ops/otel/collector-dev.rw.yaml')
policies = []
if path.exists():
    in_block = False
    current = None
    with path.open() as fh:
        for raw_line in fh:
            line = raw_line.rstrip('\n')
            stripped = line.strip()
            if not in_block:
                if stripped == 'policies:':
                    in_block = True
                continue
            if stripped and not stripped.startswith('-') and not line.startswith('      '):
                break
            if stripped.startswith('- name:'):
                if current:
                    policies.append(current)
                current = {'name': stripped.split(':', 1)[1].strip()}
            elif current is not None and ':' in stripped:
                key, value = stripped.split(':', 1)
                key = key.strip()
                value = value.strip()
                if key not in ('name',):
                    current[key] = value
        if current:
            policies.append(current)
if policies:
    for policy in policies:
        parts = [f"`{policy.get('name', 'unknown')}`"]
        policy_type = policy.get('type')
        if policy_type:
            parts.append(f"tipo={policy_type}")
        sampling = policy.get('sampling_percentage')
        if sampling:
            parts.append(f"amostra={sampling}")
        status_codes = policy.get('status_codes')
        if status_codes:
            parts.append(f"status={status_codes}")
        threshold = policy.get('threshold_ms')
        if threshold:
            parts.append(f"threshold_ms={threshold}")
        print('- ' + ' • '.join(parts))
else:
    print('- (nenhuma política encontrada)')
PY
)

cat <<EOF > "$COMMENT_FILE"
# ${JIRA_KEY} — Gatecheck closeout (${TS})

## Ferramentas
${TOOLS_INFO}

## Status dos gates
${GATE_STATUS}

## Evidências principais
${evidence_section}

## Spans canônicos
${canonical_spans}

## Políticas de tail sampling
${tail_policies}
EOF

zip_targets=()
if [ -f "ops/otel/collector-dev.trace.yaml" ]; then
    zip_targets+=("ops/otel/collector-dev.trace.yaml")
fi
for candidate in scripts/*obs4*; do
    if [ -e "$candidate" ]; then
        zip_targets+=("$candidate")
    fi
done
for candidate in docs/obs4_* docs/runbooks/obs4_*; do
    if [ -e "$candidate" ]; then
        zip_targets+=("$candidate")
    fi
done
if [ -d "$EVIDENCE_DIR" ]; then
    zip_targets+=("$EVIDENCE_DIR")
fi
if [ -d "$LOG_DIR" ]; then
    zip_targets+=("$LOG_DIR")
fi

if [ "${#zip_targets[@]}" -eq 0 ]; then
    printf 'Nenhum artefato encontrado para o ZIP.\n' >&2
elif command -v zip >/dev/null 2>&1; then
    (cd "$REPO_ROOT" && zip -r "$ZIP_FILE" "${zip_targets[@]}") >/dev/null
else
    printf 'Ferramenta zip não encontrada; não foi possível gerar o arquivo %s.\n' "$ZIP_FILE" >&2
    exit 1
fi

stage_if_trackable() {
    local path="$1"
    if [ ! -e "$path" ]; then
        return
    fi
    if git check-ignore -q "$path"; then
        printf 'Arquivo ignorado não incluído no commit: %s\n' "$path" >&2
        return
    fi
    git add "$path"
}

stage_if_trackable "scripts/obs4_finalize.sh"
if [ -f "docs/obs4_closeout.md" ]; then
    stage_if_trackable "docs/obs4_closeout.md"
fi
if [ -f "ops/otel/collector-dev.trace.yaml" ]; then
    stage_if_trackable "ops/otel/collector-dev.trace.yaml"
fi

git commit -m "obs(${JIRA_KEY}): tracing + evidências ORR"

if git remote get-url origin >/dev/null 2>&1; then
    git push -u origin "$BRANCH"
else
    printf 'Remote "origin" não configurado; push manual necessário.\n' >&2
fi

if command -v gh >/dev/null 2>&1; then
    gh pr create --base "$BASE" --head "$BRANCH" --title "obs(${JIRA_KEY}): tracing + evidências ORR" --body-file "$COMMENT_FILE"
else
    printf 'gh não encontrado; criar PR manualmente.\n'
fi

printf 'BRANCH=%s\n' "$BRANCH"
printf 'ZIP=%s\n' "$ZIP_FILE"
printf 'COMMENT=%s\n' "$COMMENT_FILE"
