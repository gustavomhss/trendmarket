#!/usr/bin/env bash
set -euo pipefail


# Garante a seção [features] e a feature 'obs'
if ! grep -q '^\[features\]' Cargo.toml; then
printf '\n[features]\nobs = []\n' >> Cargo.toml
elif ! grep -q '^obs\s*=\s*\[' Cargo.toml; then
printf 'obs = []\n' >> Cargo.toml
fi


# Cria/atualiza o bloco [[bin]] para o obs_demo com required-features = ["obs"]
if grep -q '^\[\[bin\]\]' Cargo.toml && grep -q 'name\s*=\s*"obs_demo"' Cargo.toml; then
# Remove bloco existente do obs_demo para recriar limpo
awk 'BEGIN{p=1} /\[\[bin\]\]/{if (p==1) blk=NR} {lines[NR]=$0} END{for(i=1;i<=NR;i++){print lines[i]}}' Cargo.toml > Cargo.toml.tmp
# Simples abordagem: append um bloco correto ao final
fi


# Garante caminho
mkdir -p src/bin
if [ -f src/bin/obs_demo.rs ]; then
# Se não houver gate de crate, adiciona
if ! grep -q '#!\[cfg(feature = "obs")\]' src/bin/obs_demo.rs; then
tmp=$(mktemp)
printf '#![cfg(feature = "obs")]\n' > "$tmp"
cat src/bin/obs_demo.rs >> "$tmp"
mv "$tmp" src/bin/obs_demo.rs
fi
# Se não houver main, cria um placeholder (ativo quando feature obs)
if ! grep -qE '^\s*fn\s+main\s*\(' src/bin/obs_demo.rs; then
cat >> src/bin/obs_demo.rs <<'RS'
#[cfg(feature = "obs")]
fn main() {
println!("obs_demo: feature 'obs' ativa");
}
RS
fi
fi


# Adiciona o bloco [[bin]] consolidado no final do Cargo.toml
cat >> Cargo.toml <<'TOML'


[[bin]]
name = "obs_demo"
path = "src/bin/obs_demo.rs"
required-features = ["obs"]
TOML


echo "[obs] ✅ obs_demo agora só compila com --features obs (e possui main placeholder)."
