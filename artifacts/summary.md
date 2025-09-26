# Invariant Run Summary

- Timestamp (UTC): 2025-09-20T04:33:19Z
- Duration: 5s
- Signal: none
- [P1] cargo nextest run --all --all-features --no-fail-fast --profile ci: exit 0
- [P2] cargo test --all --all-features -- --nocapture: exit 0
- Gate should fail: false

Artifacts:
- junit: artifacts/junit.xml
- cargo log: artifacts/cargo-test.log
- exit codes: artifacts/exitcodes.env
