# OpenTelemetry Collector Contrib Distro

This distribution contains all the components from both the [OpenTelemetry Collector](https://github.com/open-telemetry/opentelemetry-collector) repository and the [OpenTelemetry Collector Contrib](https://github.com/open-telemetry/opentelemetry-collector-contrib) repository. This distribution includes open source and vendor supported components.

## Recommendation

As this distribution contains many components, it is a good starting point to try various configurations. However, when running in production, it is recommended to limit the collector to contain only the components necessary for an environment. Some reasons to do this:

* reduce the size of the collector, reducing deployment times for the collector
* improve the security of the collector by reducing the available attack surface area

Building a [custom collector](https://opentelemetry.io/docs/collector/custom-collector/) can be achieved using the [OpenTelemetry Collector Builder](https://github.com/open-telemetry/opentelemetry-collector/tree/main/cmd/builder).

## Components

The full list of components is available in the [manifest](manifest.yaml)

### Rules for Component Inclusion

- Include all extensions at [Alpha stability](https://github.com/open-telemetry/opentelemetry-collector#alpha) or higher and pipeline components that have at least 1 signal at [Alpha stability](https://github.com/open-telemetry/opentelemetry-collector#alpha) or higher.

## Operational Governance: Watchers & Gate A110

This repository now ships with first-class governance artifacts to guarantee that the mandatory watchers and A110 hooks defined in `agents.md` stay green across environments.

### Inventory

- **Watchers:** Domain-specific configurations live under [`ops/watchers/`](ops/watchers). Each file describes the KPI, window, action, owner, and rollback policy for the mandatory watches covering DEC, PM, ML, DATA, PLAT, FE, SEC/PRIV and INT domains.
- **Gate A110 hooks:** The consolidated mapping is defined in [`ops/hooks/a110.yml`](ops/hooks/a110.yml). Every watcher is wired to the correct A110 hook, including the required thresholds and evidence links.

### Local enablement

1. Ensure Python 3.11+ is available (the same interpreter used for the existing automation).
2. Run the dry-run validators before coding:

   ```bash
   make watchers.dry
   make hooks.dry
   ```

   Both commands surface missing watchers, invalid parameters, or mismatched hook bindings with actionable error messages.
3. For a full Gate A110 rehearsal (watchers + hooks + Rust invariants), execute:

   ```bash
   make gate.a110
   ```

   The script aborts immediately if any watcher or hook validation fails, ensuring issues are detected before the Rust suites start.

### CI guidance

- The canonical A110 pipeline (`scripts/a110_run_invariants.sh`) now invokes `watchers.dry` and `hooks.dry` automatically. Any gap in coverage causes the gate to fail with a non-zero exit code, guaranteeing enforcement during pull requests.
- Integrate the Make targets above (or call the scripts directly) in bespoke CI systems to keep behaviour consistent between local, nightly, and production promotion flows.
- When adding a new watcher or hook, commit the YAML changes together with updated documentation so that the dry-run validators remain the single source of truth.
