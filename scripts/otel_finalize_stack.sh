#!/usr/bin/env bash
set -euo pipefail

echo "➕ Garantindo deps base…"
cargo add anyhow@=1 -q || true
cargo add tracing@=0.1.40 -q || true
cargo add tracing-subscriber@=0.3.18 --features env-filter,fmt,registry,tracing-log -q || true
cargo add tokio@=1 -F macros,rt-multi-thread,time -q || true

echo "➕ Adicionando/pinando OpenTelemetry (0.30.0) + bridge (0.31.0)…"
cargo add opentelemetry@=0.30.0 -q
cargo add opentelemetry_sdk@=0.30.0 --features trace,metrics,rt-tokio -q
cargo add opentelemetry-otlp@=0.30.0 --features trace,metrics,http-proto -q
cargo add opentelemetry-http@=0.30.0 -q
cargo add opentelemetry-proto@=0.30.0 -q
cargo add tracing-opentelemetry@=0.31.0 -q

echo "🧹 Regenerando lock e alinhando prost em 0.13.5…"
rm -f Cargo.lock
cargo generate-lockfile
cargo update -p prost --precise 0.13.5 || true
cargo update -p prost-types --precise 0.13.5 || true
cargo update -p prost-derive --precise 0.13.5 || true

echo "🔎 Checando se sobrou prost 0.14.*…"
if cargo tree --color=never -i 'prost@0.14.*' | grep -q 'prost v0\.14'; then
  echo "❌ prost 0.14.* ainda presente. Quem puxa é:"
  cargo tree --color=never -i 'prost@0.14.*' || true
  exit 2
fi

echo "🔎 Snapshot do stack OTel:"
cargo tree --color=never | grep -E 'opentelemetry(-sdk|-otlp|-http|-proto)? v0\.30\.0|tracing-opentelemetry v0\.31\.0' || true

echo "🛠️ cargo check…"
cargo check -q
echo "✅ Deps OK e código compila (check)."
