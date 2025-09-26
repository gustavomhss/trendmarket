#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
python3 - "$T" <<'PY'
import io,re,sys
p=sys.argv[1]
s=io.open(p,encoding='utf-8').read()
if not re.search(r'^\s*\[features\]\s*$',s,re.M):
    s=s.rstrip('\n')+"\n\n[features]\ndefault = []\nobs = []\nobs-bin = []\n"
else:
    m=re.search(r'^\s*\[features\]\s*$',s,re.M)
    start=m.end()
    n=re.search(r'^\s*\[[^]]+\]\s*$',s[start:],re.M)
    end=start+(n.start() if n else len(s)-start)
    body=s[start:end]
    need=[]
    for k in ('default','obs','obs-bin'):
        if not re.search(rf'^\s*{re.escape(k)}\s*=',body,re.M):
            need.append(f'{k} = []\n')
    if need:
        s=s[:end]+''.join(need)+s[end:]
io.open(p,'w',encoding='utf-8').write(s)
print("OK")
PY
echo OK
