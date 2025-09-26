#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"

python3 - "$T" <<'PY'
import io, re, sys
p = sys.argv[1]
s = io.open(p, encoding='utf-8').read()

parts = re.split(r'(^\s*\[dependencies\]\s*$)', s, flags=re.M)
keep = []
i = 0
while i < len(parts):
    if parts[i].strip().startswith('[dependencies]'):
        i += 2
    else:
        keep.append(parts[i])
        i += 1
s = ''.join(keep)

if not re.search(r'^\s*\[features\]\s*$', s, flags=re.M):
    s = s.rstrip('\n') + "\n\n[features]\ndefault = []\nobs = []\nobs-bin = []\n"
else:
    m = re.search(r'^\s*\[features\]\s*$', s, flags=re.M)
    start = m.end()
    n = re.search(r'^\s*\[[^]]+\]\s*$', s[start:], flags=re.M)
    end = start + (n.start() if n else len(s) - start)
    body = s[start:end]
    add = []
    for k in ('default','obs','obs-bin'):
        if not re.search(r'^\s*'+re.escape(k)+r'\s*=\s*', body, flags=re.M):
            add.append(f"{k} = []\n")
    if add:
        s = s[:end] + ''.join(add) + s[end:]

deps = {
    'anyhow': '"1"',
    'once_cell': '"1"',
    'opentelemetry': '"0.26"',
    'opentelemetry-jaeger': '"0.20"',
    'opentelemetry-prometheus': '"0.15"',
    'opentelemetry_sdk': '"0.26"',
    'prometheus': '"0.13"',
    'tiny_http': '"0.12"',
    'tracing': '"0.1"',
    'tracing-opentelemetry': '"0.27"',
    'tracing-subscriber': '{ version = "0.3", features = ["env-filter","fmt","registry","tracing-log","ansi"] }',
    'uuid': '{ version = "1", features = ["v4"] }',
}

s = re.sub(r'\n+$','\n', s)
s += "\n[dependencies]\n"
for k in sorted(deps):
    s += f"{k} = {deps[k]}\n"

if 'name = "obs_demo"' not in s:
    s += '\n[[bin]]\nname = "obs_demo"\npath = "src/bin/obs_demo.rs"\nrequired-features = ["obs-bin"]\n'

io.open(p,'w',encoding='utf-8').write(s)
print("OK")
PY

echo "OK: Cargo.toml pinned (backup em Cargo.toml.bak)"
