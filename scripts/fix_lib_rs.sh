#!/usr/bin/env bash
set -euo pipefail
mods=(amm types math util)
echo "/* auto-generated lib.rs */" > src/lib.rs
for m in "${mods[@]}"; do
  if [ -f "src/$m.rs" ] || [ -d "src/$m" ]; then
    echo "pub mod $m;" >> src/lib.rs
  fi
done
