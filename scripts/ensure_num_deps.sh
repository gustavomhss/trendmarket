#!/usr/bin/env bash
set -euo pipefail
have_add="no"; cargo add -V >/dev/null 2>&1 && have_add="yes"

add_dep() { # add_dep <crate> <ver>
  local crate="$1" ver="$2"
  if ! grep -qE "^[[:space:]]*${crate}[[:space:]]*=" Cargo.toml; then
    if [ "$have_add" = "yes" ]; then
      cargo add "${crate}@${ver}"
    else
      grep -q '^\[dependencies\]' Cargo.toml || printf '\n[dependencies]\n' >> Cargo.toml
      printf '%s = "%s"\n' "$crate" "$ver" >> Cargo.toml
    fi
  fi
}

add_dep num-bigint 0.4
add_dep num-integer 0.1
add_dep num-rational 0.4
