#!/usr/bin/env bash
set -euo pipefail
F="Cargo.toml"
python3 - "$F" <<'PY'
import io, re, sys
p = sys.argv[1]
s = io.open(p, encoding='utf-8').read()

def rebuild_deps(txt):
    pat = re.compile(r'(?ms)^\s*\[dependencies\]\s*\n(.*?)(?=^\s*\[[^]]+\]\s*$|^\s*\[\[[^]]+\]\]\s*$|\Z)')
    illegal = {'name','path','required-features','test','doctest','bench'}
    out = []
    last = 0
    for m in pat.finditer(txt):
        out.append(txt[last:m.start()])
        body = m.group(1)
        cleaned = []
        for ln in body.splitlines(True):
            ln_stripped = ln.strip()
            if not ln_stripped or ln_stripped.startswith('#'):
                cleaned.append(ln); continue
            kv = re.match(r'^([A-Za-z0-9_\-]+)\s*=', ln_stripped)
            if kv and kv.group(1) in illegal:
                continue
            cleaned.append(ln)
        out.append('[dependencies]\n')
        out.append(''.join(cleaned))
        last = m.end()
    out.append(txt[last:])
    return ''.join(out)

def ensure_features(txt):
    if not re.search(r'(?m)^\s*\[features\]\s*$', txt):
        return txt.rstrip('\n') + '\n\n[features]\ndefault = []\nobs = []\nobs-bin = []\n'
    m = re.search(r'(?m)^\s*\[features\]\s*$', txt)
    start = m.end()
    n = re.search(r'(?m)^\s*\[[^]]+\]\s*$|^\s*\[\[[^]]+\]\]\s*$', txt[start:])
    end = start + (n.start() if n else len(txt) - start)
    body = txt[start:end]
    add = []
    for k in ('default','obs','obs-bin'):
        if not re.search(rf'(?m)^\s*{re.escape(k)}\s*=', body):
            add.append(f'{k} = []\n')
    if add:
        txt = txt[:end] + ''.join(add) + txt[end:]
    return txt

def ensure_obs_demo_bin(txt):
    pat = re.compile(r'(?ms)^\s*\[\[bin\]\]\s*\n(.*?)(?=^\s*\[[^]]+\]\s*$|^\s*\[\[[^]]+\]\]\s*$|\Z)')
    found = False
    out = []
    last = 0
    for m in pat.finditer(txt):
        out.append(txt[last:m.start()])
        body = m.group(1)
        name = re.search(r'(?m)^\s*name\s*=\s*"([^"]+)"\s*$', body)
        if name and name.group(1) == 'obs_demo':
            found = True
            b = re.sub(r'(?m)^\s*required-features\s*=.*\n?', '', body)
            if not re.search(r'(?m)^\s*path\s*=\s*".+"', b):
                b += 'path = "src/bin/obs_demo.rs"\n'
            if not re.search(r'(?m)^\s*required-features\s*=\s*\[', b):
                b += 'required-features = ["obs-bin"]\n'
            out.append('[[bin]]\n' + b)
        else:
            out.append('[[bin]]\n' + body)
        last = m.end()
    out.append(txt[last:])
    txt = ''.join(out)
    if not found:
        txt = txt.rstrip('\n') + '\n\n[[bin]]\nname = "obs_demo"\npath = "src/bin/obs_demo.rs"\nrequired-features = ["obs-bin"]\n'
    return txt

s = rebuild_deps(s)
s = ensure_features(s)
s = ensure_obs_demo_bin(s)
io.open(p, 'w', encoding='utf-8').write(s)
PY
