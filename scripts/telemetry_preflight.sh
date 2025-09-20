#!/usr/bin/env bash
set -euo pipefail
command -v cargo >/dev/null || { echo "cargo não encontrado"; exit 1; }
command -v rustc >/dev/null || { echo "rustc não encontrado"; exit 1; }
if ! cargo add --version >/dev/null 2>&1; then
echo "Instalando cargo-edit..."; cargo install cargo-edit
fi
cargo --version; rustc --version; echo "cargo-edit OK"
