#!/usr/bin/env bash
set -euo pipefail


echo "== Dependentes de opentelemetry 0.28 =="
cargo tree -i opentelemetry@0.28.0 || true


echo "== Dependentes de opentelemetry_sdk 0.28 =="
cargo tree -i opentelemetry_sdk@0.28.0 || true


echo "== Dependentes de opentelemetry-otlp 0.26/0.29 =="
cargo tree -i opentelemetry-otlp@0.26.0 || true
cargo tree -i opentelemetry-otlp@0.29.0 || true


cat <<EOF


Se algum crate interno estiver pinando versões antigas, atualize seu Cargo.toml.
Se for transitive-only, tente: cargo update -p <crate> --precise <vers>.
EOF
