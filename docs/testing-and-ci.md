# Testing And CI

This document is the quick map for how Kyuubiki currently validates itself in
the `tamamono 1.x` line.

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
- `make audit-rust-lines`
  Rust source organization guard; fails when any `workers/rust/crates/**/*.rs`
  file exceeds the current `600` line ceiling
- `make audit-project-organization`
  Repository-wide organization guard; scans tracked source/docs files, keeps
  new files under the shared line ceiling, and prevents known historical debt
  files from growing further
- `make architecture-check`
  Lightweight new-architecture guard for the `1.14.x` line. It runs the
  repository organization audit, validates the docs book manifest JSON, and
  exercises the focused Operator TaskIR control-plane plus Rust live path.
- `./scripts/kyuubiki rust-line-audit`
  Same guard through the unified launcher, useful on remote hosts and CI jobs
  that do not enter through Make
- `make test-frontend`
  frontend typecheck plus production build validation
- `make workflow-preflight`
  workflow unit/topology plus browser-backed search/layout guard validation

### SDK checks

- `make test-sdk`

This runs:

- Python SDK smoke tests
- Elixir SDK smoke tests
- Rust SDK smoke tests

These tests use small local loopback fixtures and focus on:

- `AgentClient.run_study`
- result fetch
- chunk browsing

### Cross-process integration checks

- `make test-integration`
  top-level cross-process smoke suite
- `./scripts/kyuubiki headless-live-test`
  frontend-owned live headless service-executor suite
- `./scripts/kyuubiki headless-rust-live-test`
  Rust `kyuubiki-headless` live service-executor suite

The current integration family covers:

- orchestrator + Rust agents + HTTP solve flow
- temporary local control-plane boot plus real headless HTTP execution for
  `service_health`, `workflow_submit_catalog`, and `workflow_submit_graph`
- sample-backed `thermal_bar_1d`, `spring_1d`, `spring_2d`, `spring_3d`, `thermal_beam_1d`, `torsion_1d`, `heat_bar_1d`, `heat_plane_triangle_2d`, `heat_plane_quad_2d`, `frame_2d`, `frame_3d`, `truss_2d`, `truss_3d`, `plane_triangle_2d`, `plane_quad_2d`, `thermal_frame_2d`, `thermal_plane_triangle_2d`, `thermal_plane_quad_2d`, `thermal_truss_2d`, `thermal_frame_3d`, and `thermal_truss_3d` orchestrated API smoke
- protected cluster register / heartbeat / unregister flow
- frontend direct-mesh LAN agent solve and chunk flow
- Workbench UI smoke split by `Mechanical` and `Thermal / Thermo-mechanical`

The full integration entrypoint list stays in:

- [tests/integration/README.md](../tests/integration/README.md)

### Desktop shell checks

- `make test-hub-gui`
- `make test-installer-gui`
- `make test-workbench-gui`

These validate the current desktop shell family without requiring a full
desktop release build.

## Unified entry points

Use these when you want the repo to choose the right lower-level commands:

- `./scripts/kyuubiki test`
- `./scripts/kyuubiki verify`
- `./scripts/kyuubiki smoke`
- `make audit-rust-lines`

For narrower SDK or frontend-only entrypoints, use the package or Make targets
listed above.

For workflow-heavy frontend work, prefer the dedicated preflight entrypoint:

- `./scripts/kyuubiki workflow-preflight`

Start `npm run dev` inside `apps/frontend` first. The layout/search guard needs
the live benchmark route and is intentionally separate from `frontend-test`, so
plain build validation can stay fast and headless.

For service-executor and headless workflow contract changes, prefer the live
headless entrypoints before broader integration suites:

- `./scripts/kyuubiki headless-live-test`
- `./scripts/kyuubiki headless-rust-live-test`

These boot the temporary local control plane under `apps/web/test/support` and
exercise real HTTP execution instead of dry-run-only fixtures.

## CI structure

Current GitHub Actions jobs are intentionally separated:

- `web-test`
- `rust-test`
  Runs Rust formatting, workspace tests, the `600` line-count audit, and the
  medium benchmark regression gate.
- `frontend-test`
- `sdk-smoke`
- `integration-smoke-api`
- `integration-smoke-cluster`
- `integration-smoke-direct-mesh`
- `hub-gui-smoke`
- `installer-gui-smoke`
- `workbench-gui-smoke`

## Direct-mesh Docker regression lane

The repository now keeps a dedicated direct-mesh Docker regression path for the
shared LAN solver setup.

Use these entrypoints:

- `make test-integration-direct-mesh-docker`
  Run the Docker direct-mesh benchmark locally or from an operator shell.
- `make test-integration-direct-mesh-docker-compare CURRENT=tmp/direct-mesh-benchmark-container/latest/summary.json`
  Compare an existing benchmark summary against the checked-in baseline.
- `make test-integration-direct-mesh-docker-report REPEAT=3`
  Run a fresh benchmark and emit comparison artifacts beside the summary.
- `make test-integration-direct-mesh-docker-nightly`
  Run the remote `kyuubiki-lab` regression wrapper and fail on threshold regressions.

Baseline and report surfaces:

- baseline snapshot:
  [tests/integration/benchmarks/direct-mesh-docker-baseline.json](../tests/integration/benchmarks/direct-mesh-docker-baseline.json)
- local/latest benchmark output:
  `tmp/direct-mesh-benchmark-container/latest/summary.json`
- local/latest comparison report:
  `tmp/direct-mesh-benchmark-container/latest/compare.md`

Current behavior notes:

- direct-mesh Docker runtime defaults to `DOCKER_RUN_NETWORK=host`
- remote nightly execution assumes a self-hosted runner on the same LAN
- the remote lab wrapper expects a narrow passwordless sudo rule for the
  direct-mesh benchmark command path only

## Standard benchmark regression lane

The repository now also keeps a dedicated standard Rust benchmark regression
path for the checked `mechanical-core`, `thermal-core`, and `compound-core`
matrix trio.

Use these entrypoints:

- `make benchmark-standard-baselines PROFILE=10k REPEAT=3`
  Refresh the local checked baseline trio for a given standard profile tier.
- `make benchmark-standard-compare PROFILE=10k REPEAT=1`
  Run the standard matrix regression gate locally against the checked-in
  baselines.
- `make benchmark-standard-report PROFILE=10k REPEAT=1`
  Emit per-matrix reports plus one merged local standard comparison report.
- `cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix extended-physics --repeat 1`
  Run the broad physics smoke matrix for solver families that are not yet part
  of the standard 10k regression trio.
- `cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix structural-extended --repeat 1`
  Run the broad structural smoke matrix for spring, nonlinear/contact, beam,
  thermal beam, and modal frame families.
- `cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix thermal-structural --repeat 1`
  Run the coupled thermal-structural smoke matrix for thermal bar/truss/plane,
  static frame, and thermal frame families.
- `make benchmark-physics-coverage`
  Run the `1.14.x` broad physics smoke matrix across every built-in benchmark
  template. This is the quickest product-level check that the main physics
  families still have real solver execution paths before `1.15.x` and `1.16.x`
  contract work hardens engine/task formats.
- `make benchmark-standard-nightly`
  Sync the Rust workspace without `target/` to `kyuubiki-lab`, run the standard regression
  trio there, and pull the resulting reports back under `tmp/standard-benchmark/`.
- `make benchmark-profile-remote PROFILE=300k MATRIX=thermal-core REPEAT=1`
  Run a remote exploratory 300k profile smoke without requiring a checked
  baseline yet.
- `make benchmark-profile-remote PROFILE=300k MATRIX=thermal-structural REPEAT=1`
  Run a remote 300k coupled thermal-structural smoke once the medium lane is
  stable.
- `make benchmark-profile-remote PROFILE=300k MATRIX=mechanical-core CASE=axial-bar-300k REPEAT=1`
  Run a narrow 300k mechanical probe before attempting a full mechanical
  matrix.
- `make benchmark-profile-remote PROFILE=300k MATRIX=mechanical-core CASE=truss-roof-300k REPEAT=1 SOLVER_PRECONDITIONER=all`
  Run the truss solver strategy probe with both Jacobi and symmetric
  Gauss-Seidel preconditioners. Use `jacobi` or `symmetric-gauss-seidel` to
  force one strategy.

Baseline and report surfaces:

- overview ladder:
  [workers/rust/benchmarks/BASELINE-OVERVIEW.md](../workers/rust/benchmarks/BASELINE-OVERVIEW.md)
- checked baseline family:
  `workers/rust/benchmarks/*-core-<profile>-baseline.json`
- local/latest merged report:
  `tmp/standard-benchmark/<slug>/standard-<profile>-compare.md`
- local/latest per-matrix reports:
  `tmp/standard-benchmark/<slug>/*-core-<profile>-compare.md`
- exploratory profile smoke output:
  `tmp/benchmark-profile/<slug>/<matrix>-<profile>.json` plus a generated
  `README.md`; truss cases include solver preconditioner, iteration count, and
  residual norm when available
- local run index:
  `tmp/standard-benchmark/index.json`, `tmp/standard-benchmark/README.md`, and
  `tmp/standard-benchmark/index.html`

Current behavior notes:

- local laptop runs are useful for functional regression gates, but reference
  timing should prefer `kyuubiki-lab`
- the current nightly lane is intentionally anchored at `PROFILE=10k` and
  `REPEAT=1` so it stays stable and affordable as a first always-on signal
- `200k` and `300k` are remote-first: CI checks the catalog shape, while timing evidence
  should be collected from `kyuubiki-lab` before adding checked baselines
- cases under `5.0 ms` baseline median remain visible in reports but are not
  treated as hard failures by default
- the remote wrapper syncs the Rust workspace without `target/` and does not
  rely on checked-in server-specific runtime configuration files
- local retained run folders are now indexed and pruned by retention count so
  nightly artifact history does not sprawl indefinitely on the runner workspace

## Nightly lane map

Current self-hosted nightly flows have distinct jobs:

- direct-mesh Docker nightly:
  end-to-end LAN direct-mesh regression through the Docker harness
- workflow catalog nightly:
  orchestrated composite workflow regression through the Elixir catalog path
- standard benchmark nightly:
  solver-family performance regression for the standard Rust matrix trio on the
  reference lab machine

Local nightly artifacts are also indexed together under:

- `tmp/nightly-overview.json`
- `tmp/nightly-overview.html`

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
- workflow builder or operator-search UI change:
  `make workflow-preflight`
- orchestrator behavior change:
  `make test-web && make test-integration-api`
- SDK-only change:
  `make test-sdk`
- desktop-shell change:
  `make test-workbench-gui`
