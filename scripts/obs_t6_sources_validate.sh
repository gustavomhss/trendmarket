#!/usr/bin/env bash
set -Eeuo pipefail
mkdir -p out/obs_gatecheck/evidence
if python3 scripts/obs_sources_validate.py; then
  echo "SOURCES_OK"
else
  echo "SOURCES_FAIL:validation"
  exit 5
fi
