#!/usr/bin/env bash
set -euo pipefail

cargo test --features obs --test telemetry_spans_cdc_tests -- span_exports_attributes_via_in_memory_exporter --nocapture
