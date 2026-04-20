# Development Notes

## Current State

This repository is no longer a thin scaffold. It contains:

- a Next.js workbench
- an Elixir orchestrator API
- a Rust engine/solver workspace
- a Tauri installer GUI
- local, cloud, and distributed deployment modes

The default development style remains TDD-first. New behavior should start with
a test before production code changes.

## Repository Conventions

- Put BEAM application code under `apps/web`
- Put browser UI code under `apps/frontend`
- Put installer GUI code under `apps/installer-gui`
- Keep shared contracts in `schemas`
- Keep deployment descriptors in `deploy`
- Keep Rust crates under `workers/rust/crates`
- Treat result artifacts (`uploads/`, `storage/`, `artifacts/`, `checkpoints/`)
  as runtime output, not source-controlled assets
- Treat `tmp/` and `dist/` as generated/runtime directories
- Reach first for `make tdd-web` or `make tdd-rust` instead of editing code
  without a failing test

## Unified Entry Point

Use `./scripts/kyuubiki` as the top-level local launcher.

- `./scripts/kyuubiki smoke` runs the current Elixir -> Rust integration flow
- `./scripts/kyuubiki sdk-smoke` runs the Python / Elixir / Rust SDK smoke suite
- `./scripts/kyuubiki frontend-test` runs frontend typecheck plus production build verification
- `./scripts/kyuubiki worker -- --job-id demo --project-id p1 --case-id c1 --steps 3`
  runs the Rust worker directly
- `./scripts/kyuubiki playground` serves the in-browser FEM playground through the
  Elixir app on `http://127.0.0.1:4000/playground/`
- `./scripts/kyuubiki frontend` serves the Next.js workbench UI on `http://127.0.0.1:3000`
- `./scripts/kyuubiki test` and `./scripts/kyuubiki verify` wrap the repo checks

This is intentionally a host-native launcher rather than a container-first one.
Right now the project is optimizing for local iteration, local IPC evolution,
and mixed-platform development more than environment isolation.

## Storage Modes

The orchestrator supports dual SQL-backed persistence:

- default local mode: `sqlite`
- shared/cloud/distributed mode: `postgres`

Example:

```bash
cd /Users/Shared/chroot/dev/kyuubiki
make start-local
```

For cloud/distributed:

```bash
KYUUBIKI_STORAGE_BACKEND=postgres \
DATABASE_URL=ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev \
make start-cloud
```

In distributed mode:

```bash
KYUUBIKI_DEPLOYMENT_MODE=distributed \
KYUUBIKI_AGENT_DISCOVERY=registry \
make start-distributed
```

The control plane can then accept remote agent registration and heartbeat.

## Containerization Fit

Containerization is useful here, but not as the default local development mode
yet.

Good fits:

- CI
- Linux-based remote workers
- future distributed execution nodes
- reproducible deployment packaging

Less ideal right now:

- macOS/Windows local IPC experiments
- frontend visualization work that may want direct host graphics/browser access
- early-stage debugging where Elixir and Rust processes are evolving quickly

The current recommendation is host-native development first, containers later
for worker deployment boundaries.

## TDD Workflow

1. Write the smallest failing test for one behavior
2. Make it pass with the minimum implementation
3. Refactor with the test suite still green
4. Run `make verify` before wrapping the change

## Active Development Priorities

1. Push the Rust solver path toward stable `10k`-node single-machine targets
2. Expand distributed orchestration and remote solver deployment flows
3. Keep the frontend chunk-aware and viewport-driven for larger models
4. Preserve engine-style decoupling between browser, control plane, and solver
