#!/usr/bin/env bash
set -euo pipefail
cargo test --test golden_cpmm -- --nocapture
