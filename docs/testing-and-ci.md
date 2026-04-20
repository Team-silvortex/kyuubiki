# Testing And CI

This document is the quick map for how Kyuubiki currently validates itself in
`v0.5`.

## Why the test stack is layered

Kyuubiki is no longer one program. It has:

- a browser workbench
- an Elixir control plane
- Rust solver/runtime programs
- headless SDKs
- cross-process integration paths
- desktop shells

That means one flat `test everything` command is not enough context anymore.
The repository now keeps validation split by responsibility.

## Local test layers

### Core application checks

- `make test-web`
  Elixir control-plane tests under `apps/web/test`
- `make test-rust`
  Rust workspace tests under `workers/rust`
- `make test-frontend`
  frontend typecheck plus production build validation

### SDK checks

- `make test-sdk`
- `./scripts/kyuubiki sdk-smoke`

This runs:

- Python SDK smoke tests
- Elixir SDK smoke tests
- Rust SDK smoke tests

These tests use small local loopback fixtures and focus on:

- `AgentClient.run_study`
- result fetch
- chunk browsing

### Cross-process integration checks

- `make test-integration-api`
  local orchestrator + Rust agents + HTTP solve flow
- `make test-integration-cluster`
  protected cluster register / heartbeat / unregister flow
- `make test-integration-direct-mesh`
  frontend direct-mesh LAN agent solve and chunk flow
- `make test-integration`
  all integration smoke tests

### Desktop shell checks

- `make test-workbench-gui`

This validates the Tauri workbench shell surface without requiring a full
desktop release build.

## Unified entry points

Use these when you want the repo to choose the right lower-level commands:

- `./scripts/kyuubiki test`
- `./scripts/kyuubiki verify`
- `./scripts/kyuubiki smoke`
- `./scripts/kyuubiki sdk-smoke`
- `./scripts/kyuubiki frontend-test`

## CI structure

Current GitHub Actions jobs are intentionally separated:

- `web-test`
- `rust-test`
- `frontend-test`
- `sdk-smoke`
- `integration-smoke-api`
- `integration-smoke-cluster`
- `integration-smoke-direct-mesh`
- `workbench-gui-smoke`

## Failure diagnostics

Integration jobs now provide two failure surfaces:

- uploaded `tmp/run` artifacts
- a GitHub Actions job summary with:
  - discovered runtime logs
  - high-signal error lines
  - log tail excerpts

This is meant to reduce the number of failures that require artifact download
before they become understandable.

## Recommended local sequence

For most nontrivial changes:

1. Run the smallest focused test first.
2. Run the relevant layer command.
3. Run `make verify` before wrapping the work.

Typical examples:

- UI/runtime protocol change:
  `make test-frontend && make test-sdk`
- orchestrator behavior change:
  `make test-web && make test-integration-api`
- SDK-only change:
  `make test-sdk`
- desktop-shell change:
  `make test-workbench-gui`
