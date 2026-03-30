# Development Notes

## Current State

This repository is scaffolded from the project README. It does not yet contain a
generated Phoenix application or a compiled Rust workspace.

The default development style is now TDD-first. New behavior should start with a
test before production code changes.

## Repository Conventions

- Put BEAM application code under `apps/web`
- Keep shared contracts in `schemas`
- Keep Rust crates under `workers/rust/crates`
- Treat result artifacts (`uploads/`, `storage/`, `artifacts/`, `checkpoints/`)
  as runtime output, not source-controlled assets
- Reach first for `make tdd-web` or `make tdd-rust` instead of editing code
  without a failing test

## Unified Entry Point

Use `./scripts/kyuubiki` as the top-level local launcher.

- `./scripts/kyuubiki smoke` runs the current Elixir -> Rust integration flow
- `./scripts/kyuubiki worker -- --job-id demo --project-id p1 --case-id c1 --steps 3`
  runs the Rust worker directly
- `./scripts/kyuubiki playground` serves the in-browser FEM playground on `http://127.0.0.1:8000`
- `./scripts/kyuubiki test` and `./scripts/kyuubiki verify` wrap the repo checks

This is intentionally a host-native launcher rather than a container-first one.
Right now the project is optimizing for local iteration, local IPC evolution,
and mixed-platform development more than environment isolation.

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

## Suggested Bring-Up Order

1. Create the Phoenix LiveView app in `apps/web`
2. Add PostgreSQL and Oban integration
3. Create the Rust workspace and a protocol crate in `workers/rust`
4. Validate `schemas/*.schema.json` from both runtimes
5. Add a local end-to-end job flow before attempting distributed execution

## Initial Milestones

- Milestone 1: submit a mock job and stream fake progress through LiveView
- Milestone 2: replace the mock backend with a local Rust worker process
- Milestone 3: persist result metadata and render a lightweight mesh preview
- Milestone 4: add transport abstraction for remote workers
