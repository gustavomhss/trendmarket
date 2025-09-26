#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"

python3 - "$T" <<'PY'
import io, re, sys

p = sys.argv[1]
s = io.open(p, encoding='utf-8').read()
lines = s.splitlines(True)

hdr_any  = re.compile(r'^\s*\[[^]]+\]\s*$')
hdr_deps = re.compile(r'^\s*\[dependencies\]\s*$')
hdr_bin  = re.compile(r'^\s*\[\[bin\]\]\s*$')
hdr_feat = re.compile(r'^\s*\[features\]\s*$')

out = []
i = 0
while i < len(lines):
    ln = lines[i]
    if hdr_deps.match(ln):
        out.append(ln)
        i += 1
        while i < len(lines) and not hdr_any.match(lines[i]):
            kv = lines[i].lstrip()
            if kv.startswith('name ') or kv.startswith('path ') or kv.startswith('required-features '):
                i += 1
                continue
            out.append(lines[i])
            i += 1
        continue
    out.append(ln)
    i += 1

text = ''.join(out)

if not re.search(r'^\s*\[features\]\s*$', text, re.M):
    text = text.rstrip('\n') + "\n\n[features]\ndefault = []\nobs = []\nobs-bin = []\n"
else:
    m = re.search(r'^\s*\[features\]\s*$', text, re.M)
    start = m.end()
    n = re.search(r'^\s*\[[^]]+\]\s*$', text[start:], re.M)
    end = start + (n.start() if n else len(text) - start)
    body = text[start:end]
    def inject_feature(body, k):
        return body if re.search(r'^\s*'+re.escape(k)+r'\s*=', body, re.M) else body + f"{k} = []\n"
    body = inject_feature(body, 'default')
    body = inject_feature(body, 'obs')
    body = inject_feature(body, 'obs-bin')
    text = text[:start] + body + text[end:]

has_obs_demo = False
blocks = []
i = 0
while i < len(text):
    m = re.search(r'^\s*\[\[bin\]\]\s*$', text[i:], re.M)
    if not m:
        break
    bstart = i + m.end()
    blocks.append((i + m.start(), bstart))
    n = re.search(r'^\s*\[[^]]+\]\s*$', text[bstart:], re.M)
    bend = bstart + (n.start() if n else len(text) - bstart)
    block = text[bstart:bend]
    if re.search(r'^\s*name\s*=\s*"obs_demo"\s*$', block, re.M):
        has_obs_demo = True
        body = re.sub(r'^\s*required-features\s*=.*$', '', block, flags=re.M)
        if not re.search(r'^\s*path\s*=\s*".+"', body, re.M):
            body += 'path = "src/bin/obs_demo.rs"\n'
        if not re.search(r'^\s*required-features\s*=\s*\[', body, re.M):
            body += 'required-features = ["obs-bin"]\n'
        text = text[:bstart] + body + text[bend:]
        i = bstart + len(body)
    else:
        i = bend

if not has_obs_demo:
    text = text.rstrip('\n') + '\n\n[[bin]]\nname = "obs_demo"\npath = "src/bin/obs_demo.rs"\nrequired-features = ["obs-bin"]\n'

io.open(p, 'w', encoding='utf-8').write(text)
print('OK')
PY
