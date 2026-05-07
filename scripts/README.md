# Scripts

This directory contains host-native operational entry points.

- `kyuubiki`
  Unified launcher for local, cloud, and distributed development flows.

Use this directory for operator-facing workflow wrappers, not for source
libraries or generated output.

Typical responsibilities:

- start/stop/restart orchestration
- mode switching (`local`, `cloud`, `distributed`)
- verification/test wrappers
- component-scoped build entry points
- runtime and desktop packaging entry points
- installer entry points

Useful smoke wrappers:

- `./scripts/kyuubiki smoke`
  Current Elixir -> Rust integration smoke flow.
- `./scripts/kyuubiki sdk-smoke`
  Python / Elixir / Rust headless SDK smoke suite.
- `./scripts/kyuubiki frontend-test`
  Frontend typecheck plus production build verification.

Examples now include:

- `build-frontend`
- `build-orchestrator`
- `build-agent`
- `build-hub-gui`
- `build-installer-gui`
- `build-workbench-gui`
- `package-runtime`
- `package-desktop`
- `desktop-stage`
- `desktop-build-host`
- `desktop-release`
- `desktop-verify`
- `sync-desktop-shared`
- `test-hub-gui`
- `test-installer-gui`
- `test-workbench-gui`

Keep these scripts thin. Product logic should live in the application/runtime
code, not in shell branching.
