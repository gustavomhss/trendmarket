#!/usr/bin/env bash
set -Eeuo pipefail
! grep -RIn "^<<<<<<<\|^=======\|^>>>>>>>" -n . || { echo "ERRO: Conflitos"; exit 2; }
! grep -RInE '\\.\\.\\.|TBD|FIXME' -n .github/workflows || { echo "ERRO: Placeholder no CI"; exit 3; }
