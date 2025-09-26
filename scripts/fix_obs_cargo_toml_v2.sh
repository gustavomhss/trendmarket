#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"

python3 - "$T" > "$T.tmp" <<'PY'
import sys, re, io

path = sys.argv[1]
s = io.open(path, encoding='utf-8').read()

deps = {}

def merge_deps(block: str):
    for line in block.splitlines():
        m = re.match(r'\s*([A-Za-z0-9_\-]+)\s*=\s*(.+)\s*$', line)
        if m:
            k, v = m.group(1), m.group(2)
            deps[k] = f"{k} = {v}"

out_chunks = []
i = 0
while i < len(s):
    m = re.search(r'^\s*\[[^]]+\]\s*$', s[i:], re.M)
    if not m:
        out_chunks.append(s[i:])
        break
    hdr_start = i + m.start()
    hdr_end = i + m.end()
    out_chunks.append(s[i:hdr_start])
    hdr = s[hdr_start:hdr_end]
    sec = re.match(r'^\s*\[([^\]]+)\]\s*$', hdr, re.M).group(1).strip()
    n = re.search(r'^\s*\[[^]]+\]\s*$', s[hdr_end:], re.M)
    sec_end = hdr_end + (n.start() if n else len(s) - hdr_end)
    body = s[hdr_end:sec_end]
    if sec == 'dependencies':
        merge_deps(body)
    else:
        out_chunks.append(hdr + body)
    i = sec_end

overrides = {
    "tracing": 'tracing = "0.1"',
    "tracing-subscriber": 'tracing-subscriber = { version = "0.3", features = ["env-filter","fmt","registry","tracing-log","ansi"] }',
    "tracing-opentelemetry": 'tracing-opentelemetry = "0.27"',
    "opentelemetry": 'opentelemetry = "0.23"',
    "opentelemetry_sdk": 'opentelemetry_sdk = { version = "0.23", features = ["metrics"] }',
    "opentelemetry-prometheus": 'opentelemetry-prometheus = "0.15"',
    "opentelemetry-jaeger": 'opentelemetry-jaeger = "0.20"',
    "prometheus": 'prometheus = "0.13"',
    "once_cell": 'once_cell = "1"',
    "tiny_http": 'tiny_http = "0.12"',
    "anyhow": 'anyhow = "1"',
    "uuid": 'uuid = { version = "1", features = ["v4"] }',
}
deps.update(overrides)

out = ''.join(out_chunks)
if not out.endswith('\n'):
    out += '\n'
out += '\n[dependencies]\n'
for k in sorted(deps):
    out += deps[k] + '\n'

sys.stdout.write(out)
PY

mv "$T.tmp" "$T"
echo "OK: Cargo.toml consolidado (backup em Cargo.toml.bak)"
