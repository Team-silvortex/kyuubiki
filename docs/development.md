# Development Notes

Use this document for the day-to-day contributor workflow: where to start,
which launcher to reach for, and how to keep changes aligned with the current
repo shape.

Use these companion docs when you need deeper detail:

- [repository-structure.md](repository-structure.md)
  directory ownership and generated-path boundaries
- [testing-and-ci.md](testing-and-ci.md)
  verification entrypoints and smoke layout
- [operations.md](operations.md)
  deployment modes, runtime switches, and operator flows
- [frontend-style.md](frontend-style.md)
  visual/UI language
- [frontend-implementation.md](frontend-implementation.md)
  frontend component, state, and helper boundaries

## Working posture

The default development style remains TDD-first. New behavior should start with
a failing test before production code changes.

Use these baseline docs to stay aligned:

- [philosophy.md](philosophy.md)
- [current-line.md](current-line.md)

## Repository conventions

- Put BEAM application code under `apps/web`
- Put browser UI code under `apps/frontend`
- Put shared desktop UI helpers under `apps/desktop-shared`
- Put hub GUI code under `apps/hub-gui`
- Put installer GUI code under `apps/installer-gui`
- Put workbench GUI code under `apps/workbench-gui`
- Keep shared contracts in `schemas`
- Keep deployment descriptors in `deploy`
- Keep Rust crates under `workers/rust/crates`
- Treat result artifacts (`uploads/`, `storage/`, `artifacts/`, `checkpoints/`)
  as runtime output, not source-controlled assets
- Treat `tmp/` and `dist/` as generated/runtime directories
- Reach first for `make tdd-web` or `make tdd-rust` instead of editing code
  without a failing test
- Prefer extending existing tokens, contracts, and module boundaries before
  inventing parallel patterns

## Unified entry point

Use `./scripts/kyuubiki` as the top-level local launcher.

- `./scripts/kyuubiki smoke` runs the current Elixir -> Rust integration flow
- `./scripts/kyuubiki sdk-smoke` runs the Python / Elixir / Rust SDK smoke suite
- `./scripts/kyuubiki frontend-test` runs frontend typecheck plus production build verification
- `./scripts/kyuubiki hot-local` runs the full local dev stack with restart-on-change
- `./scripts/kyuubiki hot-cloud` runs the full cloud/postgres dev stack with restart-on-change
- `./scripts/kyuubiki hot-distributed` runs the distributed control-plane dev loop with restart-on-change
- `./scripts/kyuubiki hot-web` watches and restarts the Elixir control plane
- `./scripts/kyuubiki hot-agent` watches and restarts the Rust solver agent
- `./scripts/kyuubiki worker -- --job-id demo --project-id p1 --case-id c1 --steps 3`
  runs the Rust worker directly
- `./scripts/kyuubiki playground` serves the in-browser FEM playground through the
  Elixir app on `http://127.0.0.1:4000/playground/`
- `./scripts/kyuubiki frontend` serves the Next.js workbench UI on `http://127.0.0.1:3000`
- `./scripts/kyuubiki test` and `./scripts/kyuubiki verify` wrap the repo checks

This is intentionally a host-native launcher rather than a container-first one.
Right now the project is optimizing for local iteration, local IPC evolution,
and mixed-platform development more than environment isolation.

## Default loops

The shortest useful loops are:

- `make hot-local`
  full local workstation loop
- `make hot-cloud`
  full PostgreSQL-backed cloud loop
- `make hot-distributed`
  distributed control-plane loop
- `./scripts/kyuubiki test`
  broad repository test pass
- `./scripts/kyuubiki verify`
  higher-confidence pre-merge check

For exact runtime/storage mode details, use
[operations.md](operations.md).

## TDD Workflow

1. Write the smallest failing test for one behavior
2. Make it pass with the minimum implementation
3. Refactor with the test suite still green
4. Run `make verify` before wrapping the change

Prefer these first targets when choosing the first failing test:

- `Elixir`
  job validation and lifecycle transitions, orchestration-side progress
  application, worker adapter behavior, and UI/control-plane boundary cases
- `Rust`
  protocol/domain invariants, solver progress semantics, transport parsing,
  checkpoint/retry, and cancellation edge cases

Keep the loop behavior-focused:

- one failing test should correspond to one behavior change
- prefer behavior tests over implementation-detail tests
- reproduce a bug with a regression test before fixing it
- when shared contracts move, update both runtimes if both consume them

Fast verification shortcuts:

- `make tdd-web FILE=test/kyuubiki_web/jobs/store_test.exs`
- `make tdd-rust FILTER=protocol`
- `make test`
- `make verify`

## Active development priorities

1. Push the Rust solver path toward stable `10k`-node single-machine targets
2. Expand distributed orchestration and remote solver deployment flows
3. Keep the frontend chunk-aware and viewport-driven for larger models
4. Preserve engine-style decoupling between browser, control plane, and solver
