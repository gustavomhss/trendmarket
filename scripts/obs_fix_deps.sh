#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"; cp "$T" "$T.bak"

python3 - "$T" > "$T.tmp" <<'PY'
import sys, re, io
p = sys.argv[1]
s = io.open(p, encoding='utf-8').read()
lines = s.splitlines(True)

out = []
deps = {}

HDR_DEPS = re.compile(r'^\s*\[dependencies\]\s*(?:#.*)?$')
HDR_ANY  = re.compile(r'^\s*\[[^]]+\]\s*(?:#.*)?$')
KV       = re.compile(r'^\s*([A-Za-z0-9_\-]+)\s*=\s*(.+)$')

ILLEGAL_IN_DEPS = {
    "required-features","required_features","name","path",
    "test","bench","doctest","edition","crate-type","crate_type",
}

i = 0
while i < len(lines):
    ln = lines[i]
    if HDR_DEPS.match(ln):
        i += 1
        while i < len(lines) and not HDR_ANY.match(lines[i]):
            m = KV.match(lines[i].rstrip('\r\n'))
            if m:
                k, v = m.group(1), m.group(2)
                if k in ILLEGAL_IN_DEPS or k == "opentelemetry-jaeger":
                    i += 1; continue
                deps[k] = f"{k} = {v}"
            i += 1
        continue
    out.append(ln); i += 1

pins = {
    "tracing": 'tracing = "0.1"',
    "tracing-subscriber": 'tracing-subscriber = { version = "0.3", features = ["env-filter","fmt","registry","tracing-log","ansi"] }',
    "tracing-opentelemetry": 'tracing-opentelemetry = "=0.27.0"',
    "opentelemetry": 'opentelemetry = "=0.26.0"',
    "opentelemetry_sdk": 'opentelemetry_sdk = "=0.26.0"',
    "opentelemetry-otlp": 'opentelemetry-otlp = { version = "=0.26.0", features = ["tonic"] }',
    "opentelemetry-prometheus": 'opentelemetry-prometheus = "=0.26.0"',
    "prometheus": 'prometheus = "0.13"',
    "once_cell": 'once_cell = "1"',
    "uuid": 'uuid = { version = "1", features = ["v4"] }',
    "tiny_http": 'tiny_http = "0.12"',
    "anyhow": 'anyhow = "1"',
    "num-bigint": 'num-bigint = "0.4"',
    "num-integer": 'num-integer = "0.1"',
    "num-rational": 'num-rational = "0.4"',
    "num-traits": 'num-traits = "0.2"',
    "uint": 'uint = "0.9"',
}
deps.update(pins)

text = ''.join(out)
if not text.endswith('\n'):
    text += '\n'
text += '\n[dependencies]\n'
for k in sorted(deps):
    text += deps[k] + '\n'

sys.stdout.write(text)
PY

mv "$T.tmp" "$T"

update_to_target () {
  pkg="$1"; target="$2"
  versions="$(
    cargo tree -e no-dev 2>/dev/null \
      | grep -oE "(^|[[:space:]])${pkg} v[0-9][0-9.]*" \
      | sed -E "s/.*${pkg} v//" \
      | sort -u
  )"
  if [ -n "${versions}" ]; then
    echo "$versions" | while IFS= read -r v; do
      [ -n "$v" ] || continue
      [ "$v" = "$target" ] && continue
      cargo update -p "${pkg}@${v}" --precise "${target}" || true
    done
  fi
}

update_to_target opentelemetry            0.26.0
update_to_target opentelemetry_sdk        0.26.0
update_to_target opentelemetry-otlp       0.26.0
update_to_target opentelemetry-prometheus 0.26.0
update_to_target tracing-opentelemetry    0.27.0

cargo tree -d | grep -E "opentelemetry(_sdk|-otlp|-prometheus)?|tracing-opentelemetry" || true
echo "OK: deps unificadas (OTEL 0.26) + num-* + uint"
