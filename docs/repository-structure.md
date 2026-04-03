# Repository Structure

## Goal

This repository is organized so the product can support multiple deployment
shapes without tightly coupling the browser, orchestrator, and solver runtime.

## Top-Level Layout

- `apps/`
  Product-facing applications.
- `workers/`
  Compute/runtime crates and executables.
- `schemas/`
  Versioned JSON contracts shared across UI, orchestrator, installer, and
  solver nodes.
- `deploy/`
  Deployment descriptors such as agent manifests.
- `scripts/`
  Host-native launch and workflow entry points.
- `docs/`
  Architecture, development, and project-shape documentation.
- `tmp/`
  Local runtime state, SQLite files, and logs. Never treat this as source.
- `dist/`
  Generated portable release scaffolds.

## Application Layer

### `apps/frontend`

- Next.js workbench
- modeling UI
- result browsing
- immersive 3D editing
- installer-independent browser client

### `apps/web`

- Elixir orchestrator API
- job lifecycle
- persistence
- result chunk APIs
- watchdog and health surfaces
- distributed agent routing and registration

### `apps/installer-gui`

- Tauri installer GUI
- environment setup
- local/cloud/distributed launch flows
- remote bootstrap and remote agent control

## Compute Layer

### `workers/rust/crates/protocol`

Shared RPC messages, progress events, and solver payload types.

### `workers/rust/crates/engine`

Reusable engine-facing solve entry points and result chunk helpers.

### `workers/rust/crates/solver`

Numerical kernels and solver strategies.

### `workers/rust/crates/cli`

TCP solver agent, local worker mode, and remote self-registration runtime.

### `workers/rust/crates/benchmark`

Benchmark profiles for medium, large, v2, and 10k-scale targets.

### `workers/rust/crates/installer`

Cross-platform CLI for doctor, env validation, release staging, and deployment
setup.

## Stable Boundaries

- Frontend should depend on APIs and schemas, not Elixir internals.
- Orchestrator should depend on RPC/protocol boundaries, not UI internals.
- Solver/runtime crates should not depend on browser or Phoenix concerns.
- Deployment descriptors should live in `deploy/`, not be scattered through
  source trees.

## Runtime and Generated Paths

- `tmp/run/`
  Local logs and live process output.
- `tmp/data/`
  Local SQLite and temporary persisted runtime state.
- `dist/`
  Generated release layouts from the installer.

These paths are generated output and should remain disposable.
