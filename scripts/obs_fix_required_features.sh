#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"

python3 - "$T" <<'PY'
import io, re, sys
p = sys.argv[1]
s = io.open(p, encoding='utf-8').read()
lines = s.splitlines(True)

hdr_single = re.compile(r'^\s*\[[^]]+\]\s*$')
hdr_double = re.compile(r'^\s*\[\[[^]]+\]\]\s*$')
hdr_deps   = re.compile(r'^\s*\[dependencies\]\s*$')
hdr_feats  = re.compile(r'^\s*\[features\]\s*$')
hdr_bin    = re.compile(r'^\s*\[\[bin\]\]\s*$')

def is_any_header(l):
    return bool(hdr_single.match(l) or hdr_double.match(l))

out = []
i = 0
features_seen = False
obs_demo_seen = False

while i < len(lines):
    ln = lines[i]

    if hdr_deps.match(ln):
        j = i + 1
        body = []
        while j < len(lines) and not is_any_header(lines[j]):
            if not re.match(r'^\s*required-features\s*=', lines[j]):
                body.append(lines[j])
            j += 1
        out.append(ln)
        out.extend(body)
        i = j
        continue

    if hdr_feats.match(ln):
        features_seen = True
        j = i + 1
        body = []
        while j < len(lines) and not is_any_header(lines[j]):
            body.append(lines[j])
            j += 1
        body_txt = ''.join(body)
        need = []
        for k in ('default','obs','obs-bin'):
            if not re.search(rf'^\s*{re.escape(k)}\s*=', body_txt, re.M):
                need.append(f'{k} = []\n')
        out.append(ln)
        out.append(body_txt)
        if need:
            out.append(''.join(need))
        i = j
        continue

    if hdr_bin.match(ln):
        j = i + 1
        block = [ln]
        while j < len(lines) and not is_any_header(lines[j]):
            block.append(lines[j])
            j += 1
        body = ''.join(block[1:])

        m_name = re.search(r'^\s*name\s*=\s*"([^"]+)"\s*$', body, re.M)
        name = m_name.group(1) if m_name else None

        if name == 'obs_demo':
            obs_demo_seen = True
            body_no_req = re.sub(r'^\s*required-features\s*=.*$', '', body, flags=re.M)
            if not re.search(r'^\s*path\s*=\s*".+"', body_no_req, re.M):
                body_no_req += 'path = "src/bin/obs_demo.rs"\n'
            if not re.search(r'^\s*required-features\s*=\s*\[', body_no_req, re.M):
                body_no_req += 'required-features = ["obs-bin"]\n'
            out.append('[[bin]]\n')
            out.append(body_no_req)
        else:
            out.extend(block)

        i = j
        continue

    out.append(ln)
    i += 1

full = ''.join(out)

if not features_seen:
    full = full.rstrip('\n') + '\n\n[features]\ndefault = []\nobs = []\nobs-bin = []\n'

if 'name = "obs_demo"' not in full and not obs_demo_seen:
    full = full.rstrip('\n') + '\n\n[[bin]]\nname = "obs_demo"\npath = "src/bin/obs_demo.rs"\nrequired-features = ["obs-bin"]\n'

io.open(p, 'w', encoding='utf-8').write(full)
print("OK")
PY
