#!/usr/bin/env bash
set -Eeuo pipefail
if cargo test -q; then
  echo "TESTS_OK"
else
  echo "TESTS_FAIL:tests"
  exit 6
fi
