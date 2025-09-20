#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"

python3 - "$T" > "$T.tmp" <<'PY'
import sys, re, io
path = sys.argv[1]
s = io.open(path, encoding='utf-8').read()
lines = s.splitlines(True)
out = []
deps = {}
i = 0
hdr_deps = re.compile(r'^\s*\[dependencies\]\s*(?:#.*)?$')
hdr_any = re.compile(r'^\s*\[[^]]+\]\s*(?:#.*)?$')
kv = re.compile(r'^\s*([A-Za-z0-9_\-]+)\s*=\s*(.+)$')
while i < len(lines):
    line = lines[i]
    if hdr_deps.match(line):
        i += 1
        while i < len(lines) and not hdr_any.match(lines[i]):
            dep_line = lines[i].rstrip('\r\n')
            m = kv.match(dep_line)
            if m:
                k, v = m.group(1), m.group(2)
                deps[k] = f"{k} = {v}"
            i += 1
        continue
    out.append(line)
    i += 1
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
out_txt = ''.join(out)
if not out_txt.endswith('\n'):
    out_txt += '\n'
out_txt += '\n[dependencies]\n'
for k in sorted(deps):
    out_txt += deps[k] + '\n'
sys.stdout.write(out_txt)
PY

mv "$T.tmp" "$T"
echo "OK: Cargo.toml consolidado (backup em Cargo.toml.bak)"
