#!/usr/bin/env bash
set -euo pipefail


# 1) Garante [features] obs
if ! grep -q '^\[features\]' Cargo.toml; then
printf '\n[features]\nobs = []\n' >> Cargo.toml
elif ! grep -q '^obs\s*=\s*\[' Cargo.toml; then
printf 'obs = []\n' >> Cargo.toml
fi


# 2) Garante gate e main em src/bin/obs_demo.rs caso exista
file="src/bin/obs_demo.rs"
if [ -f "$file" ]; then
# Gate de crate
if ! grep -q '#!\[cfg(feature = "obs")\]' "$file"; then
tmp=$(mktemp); printf '#![cfg(feature = "obs")]\n' > "$tmp"; cat "$file" >> "$tmp"; mv "$tmp" "$file"
fi
# main placeholder se ausente
if ! grep -qE '^\s*fn\s+main\s*\(' "$file"; then
cat >> "$file" <<'RS'
#[cfg(feature = "obs")]
fn main() {
// placeholder: evita E0601 quando --features obs
println!("obs_demo: feature 'obs' ativa");
}
RS
fi
fi


echo "[obs] ✅ feature 'obs' garantida, gate aplicado e main criado se necessário."
