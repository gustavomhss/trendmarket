#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"


awk '
BEGIN{ in_dep=0 }
function is_section(l){ return (l ~ /^\[[^]]+\]/) }
{
if ($0 ~ /^\[dependencies\]/) { in_dep=1; next }
if (in_dep) {
if (is_section($0)) { in_dep=0; out = out $0 "\n"; next }
if ($0 ~ /^[[:space:]]*[A-Za-z0-9_\-]+[[:space:]]*=/) {
line=$0
key=$0; sub(/^[[:space:]]*/,"",key); sub(/[[:space:]=].*/,"",key)
dep[key]=line
}
next
}
out = out $0 "\n"
}
END{
# ===== Overrides obrigatórios da observabilidade =====
dep["tracing"] = "tracing = \"0.1\""
dep["tracing-subscriber"] = "tracing-subscriber = { version = \"0.3\", features = [\"env-filter\",\"fmt\",\"registry\",\"tracing-log\",\"ansi\"] }"
dep["tracing-opentelemetry"] = "tracing-opentelemetry = \"0.27\""
dep["opentelemetry"] = "opentelemetry = \"0.23\""
dep["opentelemetry_sdk"] = "opentelemetry_sdk = { version = \"0.23\", features = [\"metrics\"] }"
dep["opentelemetry-prometheus"] = "opentelemetry-prometheus = \"0.15\""
dep["opentelemetry-jaeger"] = "opentelemetry-jaeger = \"0.20\""
dep["prometheus"] = "prometheus = \"0.13\""
dep["once_cell"] = "once_cell = \"1\""
dep["tiny_http"] = "tiny_http = \"0.12\""
dep["anyhow"] = "anyhow = \"1\""
dep["uuid"] = "uuid = { version = \"1\", features = [\"v4\"] }"


print out
print "[dependencies]"
n=asorti(dep, k)
for(i=1;i<=n;i++) print dep[k[i]]
}' "$T" > "$T.tmp"


mv "$T.tmp" "$T"
echo "OK: Cargo.toml consolidado. Backup em Cargo.toml.bak"
