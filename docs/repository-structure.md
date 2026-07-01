# Repository Structure

## Goal

This repository is organized so the product can support multiple deployment
shapes without tightly coupling the browser, orchestrator, and solver runtime.

## Top-Level Layout

- `apps/`
  Product-facing applications.
- `workers/`
  Compute/runtime crates and executables.
- `sdks/`
  Headless client libraries for protocol-driven access from external tools,
  automation, and AI runtimes.
- `schemas/`
  Versioned JSON contracts shared across UI, orchestrator, installer, and
  solver nodes.
  Project bundles now standardize around an engine-style archive layout with
  `Assets/`, `ProjectSettings/`, `Workspace/`, and `Analysis/` roots inside
  `.kyuubiki` exports, plus an asset catalog and `.meta` sidecars for stable
  asset tracking, and a guid reference graph for cross-asset relations.
- `deploy/`
  Deployment descriptors such as agent manifests.
- `assets/`
  Curated brand, app icon, and dock icon source assets shared by frontend and
  desktop shells.
- `scripts/`
  Host-native launch and workflow entry points.
- `docs/`
  Architecture, development, and project-shape documentation.
- `tests/`
  Cross-process and repository-level smoke coverage that spans multiple apps.
- `tmp/`
  Local runtime state, SQLite files, and logs. Never treat this as source.
- `dist/`
  Generated portable release scaffolds.

See also:

- [project-architecture-organization.md](project-architecture-organization.md)
- [scripts/README.md](../scripts/README.md)
- [tmp/README.md](../tmp/README.md)
- [releases/README.md](../releases/README.md)
- [tests/integration/README.md](../tests/integration/README.md)

## Application Layer

### `apps/frontend`

- Next.js workbench
- modeling UI
- result browsing
- immersive 3D editing
- workflow-focused browser client
- `src/components/workbench` for domain surfaces
- `src/components/ui` for reusable UI primitives

### `apps/desktop-shared`

- shared desktop frontend helper source
- sync script for brand manifest and Tauri bridge helpers
- source-of-truth layer for the desktop Tauri app family

### `apps/web`

- Elixir orchestrator API
- job lifecycle
- persistence
- result chunk APIs
- watchdog and health surfaces
- distributed agent routing and registration
- `results/` for result persistence backends
- `storage/` for repo modules and persisted record structs
- `playground/` as the runtime/agent integration boundary
- tests mirror this split under `apps/web/test/kyuubiki_web`

### `apps/installer-gui`

- Tauri installer GUI
- environment setup
- runtime and agent deployment management
- local/cloud/distributed bootstrap flows
- update, repair, cleanup, and integrity operations
- remote bootstrap and remote agent control

### `apps/hub-gui`

- Tauri desktop hub shell
- system entrypoint and workload shell
- project launch surface
- runtime target overview and operator visibility
- intended to sit above installer and workbench in the desktop product split

### `apps/workbench-gui`

- Tauri desktop workbench shell
- native wrapper around the browser workbench
- local runtime status and log access through the shared desktop runtime crate

## Headless SDK Layer

### `sdks/python`

- stdlib-first Python SDK for control-plane HTTP and direct solver RPC access
- aimed at notebooks, local automation, and AI-driven orchestration

### `sdks/elixir`

- lightweight Elixir SDK for protocol-level access without depending on Phoenix
- suited for BEAM-side automation, service integration, and broker processes

### `sdks/rust`

- native Rust SDK for embedding control-plane and solver-RPC access into tools
- intended for headless agents, CLIs, and engine-adjacent services

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

Benchmark profiles for medium, large, v2, and 10k-scale targets, with 10k as
the default local regression tier.

### `workers/rust/crates/installer`

Cross-platform CLI for doctor, env validation, release staging, and deployment
setup.

See [workers/rust/README.md](../workers/rust/README.md)
for the crate-by-crate map.

Rust tests remain colocated with crates or under crate-local `tests/`
directories.

## Repository-Level Tests

### `tests/integration`

- cross-process smoke coverage
- launcher-driven local workstation validation
- cluster registration and heartbeat coverage
- direct-mesh frontend integration paths

## Stable Boundaries

- Frontend should depend on APIs and schemas, not Elixir internals.
- Hub, Workbench, and Installer should remain separate product surfaces even
  when they expose overlapping runtime visibility.
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
