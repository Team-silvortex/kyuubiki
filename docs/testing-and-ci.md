# Testing And CI

This document is the quick map for how Kyuubiki currently validates itself in
the `moxi 2.x` line.

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
  file exceeds the current `800` line ceiling
- `make audit-project-organization`
  Repository-wide organization guard; scans tracked files plus untracked
  files that are not ignored, keeps new files under the shared line ceiling,
  prevents known historical debt files from growing further, and keeps
  installer `tests.rs` as a module index. The Make target runs the audit
  script self-test before scanning the repository.
- `make architecture-check`
  Lightweight new-architecture guard for the `2.0.x` line. It runs the
  organization audit self-test and scan, version-line checks, UI automation
  contract checks, materialization plan contract checks, material exploration
  chain contract checks, retained material research bundle and bundle-index
  contract checks, TaskIR mirror and digest contract checks, dependency audits,
  external operator package preflight, external operator dynamic host smoke,
  docs book manifest validation, focused Operator TaskIR control-plane tests,
  and the Rust live operator task path.
- `make check-materialization-plan-contract`
  Shared materialized-candidate contract guard. It checks the materialization
  plan schema, fixture, and SDK documentation links before agent/lab output is
  treated as a solver-rerun input.
- `make check-material-exploration-chain-contract`
  Shared repeated-run material research guard. It checks the chain schema,
  fixture, convergence assessment, optimization trace, summary/run alignment,
  and documentation links before `--chain-next` output is treated as a stable
  SDK or agent-facing contract.
- `make check-material-research-bundle-index-contract`
  Shared retained-index guard. It checks the bundle-index schema, compact
  fixture, decision counts, winner-drift evidence, metric/gate summaries, and
  documentation links before generated index files are treated as lightweight
  CI, release, or agent planning artifacts.
  Keep retained material research negative fixtures in the sibling
  `*_self_test.rs` files under
  `workers/rust/crates/script-runner/src/`; the main checker modules should stay
  focused on runtime and contract logic so the 800-line source ceiling remains
  comfortable.
- `make check-ui-automation-contract`
  Product-owned Workbench UI selector contract guard. It compares
  `docs/ui-automation-contract.json`, frontend TS selector constants, and the
  component implementation anchors used by wasm-python automation and UI smoke
  tests.
- `make check-version-line`
  Shipping-version contract guard. It checks the release index, package
  metadata, generated docs mirrors, update catalogs, shipped language-pack
  catalog, and hand-maintained version-line docs against the current release
  line.
- `make check-operator-reliability`
  Operator reliability evidence guard. It verifies that every `physics-coverage`
  solve operator has a machine-readable manifest shard entry with benchmark
  coverage, headless workflow support, evidence files, trust level, and visible
  limits. It also runs a checker self-test and enforces the manifest's
  `minimum_coverage_level`, currently `qualification` for the `moxi 2.0.x`
  physics-coverage gate.
- `make audit-dependencies`
  Reproducible dependency security audit. It runs npm production dependency
  audits for the frontend and desktop packages, then RustSec `cargo audit` for
  the Rust workspace, Rust SDK, and every Tauri desktop shell. The Make target
  runs the audit lane self-test before invoking external tools. The checked
  `Cargo.lock` files under those roots are part of this contract.
- `./scripts/kyuubiki rust-line-audit`
  Same guard through the unified launcher, useful on remote hosts and CI jobs
  that do not enter through Make
- `make test-frontend`
  frontend typecheck plus production build validation
- `make workflow-preflight`
  workflow unit/topology plus browser-backed search/layout guard validation

### Installer Test Organization

Installer crate tests are split by installer responsibility instead of growing
`workers/rust/crates/installer/src/tests.rs`. Put new tests under:

- `control_update.rs` for platform parsing, agent manifests, cross-platform
  audit, and update-plan behavior
- `security_integrity.rs` for credential storage and installation integrity
  contracts
- `release_runtime.rs` for release manifests, launch manifests, embedded
  runtimes, and Linux desktop dependency plans
- `remote_deployment.rs` for remote deployment, artifact delivery, SSH fixture,
  and host trust plans
- `operator_package_preflight.rs` for external operator package admission JSON
  and quality gates

### SDK checks

- `make test-sdk`
- `make operator-package-preflight`
- `make operator-package-dynamic-smoke`
- `make check-operator-package-dynamic-smoke-contract`
- `make check-operator-package-dynamic-smoke`

This runs:

- Python SDK smoke tests
- Elixir SDK smoke tests
- Rust SDK smoke tests

The operator package preflight is a separate read-only admission check for the
external Rust operator template. It emits `kyuubiki.operator-package-preflight/v1`
JSON and confirms the package manifest, SDK API version, host version gate, and
dynamic-loading safety posture before an external package reaches runtime
activation.
Use `make operator-package-preflight OUT=tmp/operator-package-preflight.json`
when a CI job should retain the JSON report as an artifact.
Use `FAIL_ON_REJECTED=1` when rejected packages should fail the job instead of
only appearing in the report.

The dynamic smoke goes beyond read-only admission: it runs the template crate
tests, strict preflight, template `cdylib` build, and the engine dynamic host
test that loads and dispatches the template operator. It writes
`tmp/operator-package-dynamic-smoke.json` by default and accepts
`OUT=tmp/name.json` when CI should retain a named artifact.
The checker validates the retained dynamic-smoke report schema,
package/operator summary, canonical stage order, stage descriptions,
repo-local working directories, reproducible command vectors, stage success,
repo-local evidence paths, and the matching shared schema fixture under
`schemas/`.
The contract target runs the same schema/example fixture checks without
requiring a freshly generated `tmp/` report, so architecture checks can catch
contract drift before the dynamic host smoke runs.

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
- sample-backed `thermal_bar_1d`, `spring_1d`, `spring_2d`, `spring_3d`,
  `thermal_beam_1d`, `torsion_1d`, `heat_bar_1d`, `heat_plane_triangle_2d`,
  `heat_plane_quad_2d`, `frame_2d`, `frame_3d`, `solid_tetra_3d`, `truss_2d`,
  `truss_3d`, `plane_triangle_2d`, `plane_quad_2d`, `thermal_frame_2d`,
  `thermal_plane_triangle_2d`, `thermal_plane_quad_2d`, `thermal_truss_2d`,
  `thermal_frame_3d`, and `thermal_truss_3d` orchestrated API smoke
- protected cluster register / heartbeat / unregister flow
- frontend direct-mesh LAN agent solve and chunk flow
- Workbench UI smoke split by `Mechanical` and `Thermal / Thermo-mechanical`

The full integration entrypoint list stays in:

- [tests/integration/README.md](../tests/integration/README.md)

## CI lanes

- `architecture-contracts`
  Runs source organization, module topology, module-function matrix/tensor,
  shared contracts runtime API surface, lightweight runtime surface tests, UI
  automation contract, language pack, version-line, operator reliability,
  toolchain, and docs-book checks without booting long-lived services. This
  lane is meant to catch contract drift early before heavier build or
  integration jobs spend time.

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

`make verify` is the higher-confidence pre-release lane: it includes toolchain
checks, language-pack checks, version-line checks, operator reliability checks,
organization audits, dependency audits, external operator package preflight,
SDK smoke tests, and the standard benchmark gate.

For narrower SDK or frontend-only entrypoints, use the package or Make targets
listed above.

For workflow-heavy frontend work, prefer the dedicated preflight entrypoint:

- `./scripts/kyuubiki workflow-preflight`

For runtime-boundary or contract-surface changes, use the focused lightweight
surface lane:

- `make test-runtime-surfaces`

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
  Runs Rust formatting, workspace tests, the `800` line-count audit, and the
  medium benchmark regression gate.
- `dependency-audit`
  Should run `make audit-dependencies` when dependency or lockfile surfaces
  change, and before release branches are cut.
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
shared LAN solver setup. Docker-heavy direct-mesh work defaults to the
`kyuubiki-lab` server so local laptops do not become build farms. Local Docker
is still available as an explicit debug path with `LOCAL_DOCKER=1`.

Use these entrypoints:

- `make test-integration-direct-mesh-docker`
  Run the remote `kyuubiki-lab` direct-mesh Docker regression wrapper by
  default. Use `LOCAL_DOCKER=1` only for local reproduction.
- `make test-integration-direct-mesh-docker-compare CURRENT=tmp/direct-mesh-benchmark-container/latest/summary.json`
  Compare an existing benchmark summary against the checked-in baseline.
- `make test-integration-direct-mesh-docker-report REPEAT=3`
  Run the remote direct-mesh Docker regression and emit comparison artifacts.
  With `LOCAL_DOCKER=1`, run the local container and compare its summary.
- `make test-integration-direct-mesh-docker-nightly`
  Run the remote `kyuubiki-lab` regression wrapper and fail on threshold regressions.

Baseline and report surfaces:

- baseline snapshot:
  [tests/integration/benchmarks/direct-mesh-docker-baseline.json](../tests/integration/benchmarks/direct-mesh-docker-baseline.json)
- local/latest benchmark output:
  `tmp/direct-mesh-benchmark-container/latest/summary.json`
- local/latest comparison report:
  `tmp/direct-mesh-benchmark-container/latest/compare.md`

The `test-integration-remote-ssh-fixture` target remains a deliberately local
Docker fixture for SSH deployment protocol testing. It should be treated as a
small fixture, not as the default path for benchmark or release workloads.

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
- `make benchmark-compare MATRIX=mechanical-core PROFILE=10k CASE=plane-quad-panel-10k REPEAT=1`
  Run a narrow local hot-case comparison against the checked baseline. Use
  `CASE=<substring>` with `benchmark-baseline`, `benchmark-compare`,
  `benchmark-report`, or `benchmark-physics-coverage` when validating one
  suspect operator path without rerunning the full matrix. Case-filtered
  baselines and Markdown reports are written to case-suffixed artifact names so
  full-matrix baselines are not overwritten by a hot-case probe.
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
  Run the `2.0.x` broad physics smoke matrix across every built-in benchmark
  template. This is the quickest product-level check that the main physics
  families still have real solver execution paths before `2.0.x` and `2.0.x`
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
  Run the truss solver strategy probe with Jacobi, symmetric Gauss-Seidel, and
  IC(0) candidates. Use `jacobi` or `symmetric-gauss-seidel` to force one
  strategy. `ic0` selects the explicit incomplete-Cholesky candidate for large
  SPD systems; `auto` selects it for thermal-plane triangle workloads backed by
  500k and 1M evidence. Unknown names are rejected rather than silently
  falling back to Jacobi.
- `make benchmark-profile-remote PROFILE=400k MATRIX=thermal-core CASE=heat-plane-quad-400k REPEAT=1`
  Run the first remote 400k smoke as a narrow, low-risk probe before promoting
  broader matrices.
- `make benchmark-profile-remote PROFILE=400k MATRIX=mechanical-core CASE=axial-bar-400k REPEAT=1`
  Run the cheapest 400k mechanical path to confirm catalog shape and end-to-end
  runner behavior before attempting truss or full matrix coverage.
- `make benchmark-profile-remote PROFILE=400k MATRIX=mechanical-core CASE=truss-roof-400k REPEAT=1 SOLVER_PRECONDITIONER=all`
  Run the heavy 400k truss probe and compare Jacobi against symmetric
  Gauss-Seidel before choosing a default iterative-solver lane.
- `make benchmark-profile-remote PROFILE=400k MATRIX=thermal-structural CASE=thermal-plane-triangle-400k REPEAT=1 SOLVER_PRECONDITIONER=auto`
  Run the 400k thermal structural surface probe with the benchmark-selected
  thermal-plane preconditioner. `auto` keeps Jacobi for general cases, uses
  IC(0) for thermal-plane triangles and one-million-node thermal-plane quads,
  and uses symmetric Gauss-Seidel for smaller thermal-plane quads.
- `make benchmark-profile-remote PROFILE=400k MATRIX=thermal-structural CASE=thermal-plane-quad-400k REPEAT=1 SOLVER_PRECONDITIONER=auto`
  Run the matching 400k thermal quad surface probe. Current lab evidence is
  comparable to the triangle path and useful as a second FEM surface-shape
  pressure test.
- `make benchmark-profile-remote PROFILE=400k MATRIX=thermal-structural REPEAT=1 SOLVER_PRECONDITIONER=auto`
  Run the full 400k coupled thermal-structural matrix after the two surface
  probes pass. This is a long remote smoke, not a local or default nightly lane.
- `make benchmark-profile-remote PROFILE=500k MATRIX=mechanical-core CASE=axial-bar-500k REPEAT=1`
  Start 500k coverage with the cheapest remote mechanical probe. Treat 500k as
  exploratory shape coverage plus narrow lab evidence until repeated timings
  justify any scheduled matrix lane.
- `make benchmark-profile-plan PROFILE=500k`
  Print the full 500k remote-first probe plan from
  `config/benchmark-profile-coverage.json` without executing it. Use
  `MATRIX=<matrix>`, `CASE=<substring>`, and `LIMIT=<n>` to choose a safe batch.
- `SHAPES=1 make benchmark-profile-plan PROFILE=500k MATRIX=thermal-structural LIMIT=2`
  Include generated shape summaries beside each planned probe while still
  keeping the plan in dry-run mode.
- `FORMAT=json SHAPES=1 make benchmark-profile-plan PROFILE=500k MATRIX=thermal-structural LIMIT=2`
  Emit a machine-readable dry-run plan with command, output slug, and shape
  fields for later dashboards or batch controllers.
- `PLAN_OUT=tmp/benchmark-profile-plan.json SHAPES=1 make benchmark-profile-plan PROFILE=500k MATRIX=thermal-structural LIMIT=2`
  Retain the same structured dry-run plan as a repo-local JSON artifact while
  still printing the normal text plan.
- `make benchmark-shapes PROFILE=500k MATRIX=thermal-structural`
  Print generated case scale without solving. This is the fastest way to verify
  that a 500k probe is truly profile-sized before sending it to `kyuubiki-lab`.
- `make benchmark-profile-plan PROFILE=500k LIMIT=2 EXECUTE=1`
  Execute a narrowed 500k plan sequentially. Each probe gets an isolated
  `OUTPUT_SLUG`, so retained `summary.json` files can be indexed without
  per-case overwrites.
- `PROFILE=1m MATRIX=thermal-structural CASE=thermal-bar-1m REPEAT=1 ./scripts/run-benchmark-profile-remote.sh`
  Run the first exploratory one-million-node probe on `kyuubiki-lab`. Keep
  `1m` as a single-case lab stress tier for now, not a scheduled coverage gate.
- `REMOTE_TIMEOUT_SECONDS=900` is the default safety budget for every remote
  profile run. Use an explicit larger value only after the narrow probe has
  established a reason to retain the server load; timeout requests `SIGINT`
  before the final forced stop.
  Failed remote runs retain a local `failure.json` receipt with the profile,
  case, host, timeout budget, phase, semantic failure kind, exit code, and final `progress.log`
  lines. This is failure evidence, not a benchmark result.

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
  `tmp/benchmark-profile/<slug>/<matrix>-<profile>.json` plus generated
  `README.md` and `summary.json`; truss cases include solver preconditioner,
  iteration count, and residual norm when available
- exploratory profile report rebuild:
  `make benchmark-profile-report PROFILE=<profile> MATRIX=<matrix> OUTPUT_SLUG=<slug>`
  regenerates the local `README.md` from an already copied JSON report without
  SSH, rsync, or rerunning a large remote benchmark. Set `LOCAL_JSON_PATH` to
  an absolute report path when backfilling older non-standard JSON filenames.
- exploratory profile run index:
  `make benchmark-profile-index` rebuilds `tmp/benchmark-profile/index.json`
  and `tmp/benchmark-profile/README.md` from retained `summary.json` files;
  its gate is advisory and checks only for retained runs plus finite case/time/RSS
  metrics. Failed remote attempts are retained separately under `failed_runs`
  and make the advisory gate warn without being counted as coverage. Malformed
  retained summaries or failure receipts are listed under `skipped_runs` instead
  of aborting the index refresh. Matrix-level rollups are emitted under
  `matrix_summaries` for quick mechanical/thermal coverage review, and
  `coverage_summaries` tracks release-scale completeness for the standard
  `400k` and `500k` matrix contracts: `mechanical-core`, `thermal-core`,
  `compound-core`, and `thermal-structural`. Coverage targets live in
  `config/benchmark-profile-coverage.json`; use
  `./scripts/build-benchmark-profile-index.mjs --coverage-targets <manifest>`
  for experimental matrix contracts. When an older summary lacks
  `solver_preconditioners`, the index reads that run's retained raw report to
  recover `cases[].solver_preconditioner`; the manifest is validated strictly,
  so malformed or empty coverage targets fail the index refresh. Its
  `solver_strategy_summaries` compares the latest single-case observation for
  each recorded strategy without treating multi-case totals as a per-case
  measurement, including solver iterations and final residual when retained
  raw reports provide them.
- local run index:
  `tmp/standard-benchmark/index.json`, `tmp/standard-benchmark/README.md`, and
  `tmp/standard-benchmark/index.html`

Current behavior notes:

- local laptop runs are useful for functional regression gates, but reference
  timing should prefer `kyuubiki-lab`
- the 500k compound surface panel establishes the current 2D mechanical limit:
  symmetric Gauss-Seidel completed in about 67.7 seconds over 2,381 iterations,
  while an exploratory 2x2 block-Jacobi variant took about 102.1 seconds over
  7,442 iterations. Do not promote block-Jacobi; the next credible step is a
  multilevel or AMG-style preconditioner with its own validation lane.
- the compact explicit IC(0) candidate completed the same 500k panel in about
  59.9 seconds over 2,159 iterations at 2,190,268 KiB peak RSS. Its compact
  index/transpose layout reduced that panel's earlier 2,254,828 KiB peak by
  about 2.9% without changing convergence. It also completed the 1M compound
  surface panel in about 168.9 seconds over 3,061 iterations at roughly 4.4
  GiB peak RSS. Keep it opt-in outside the evidence-backed thermal-plane
  triangle auto path until additional matrix families establish a broader
  default policy.
- IC(0) also improved the 500k thermal-plane triangle from about 70.7 seconds
  and 2,544 iterations to about 66.0 seconds and 2,262 iterations. The 1M
  thermal-plane triangle subsequently completed in about 176.0 seconds over
  3,194 iterations at 5,260,592 KiB peak RSS under a reviewed 300-second
  budget. This resolves the retained 180-second attempt; keep multilevel/AMG
  work as a scalability improvement, not as a reason to relax solver
  tolerances.
- the current nightly lane is intentionally anchored at `PROFILE=10k` and
  `REPEAT=1` so it stays stable and affordable as a first always-on signal
- `200k`, `300k`, `400k`, and `500k` are remote-first: CI checks the catalog
  shape, while timing evidence should be collected from `kyuubiki-lab` before
  adding checked baselines
- cases under `5.0 ms` baseline median remain visible in reports but are not
  treated as hard failures by default
- the remote wrapper syncs the Rust workspace without `target/` and does not
  rely on checked-in server-specific runtime configuration files
- `REPORT_ONLY=1` regenerates a local profile summary without SSH when it is
  given the original `PROFILE`, `MATRIX`, and `CASE` alongside `OUTPUT_SLUG`,
  or an explicit `LOCAL_JSON_PATH`
- `CASE` selects one exact benchmark case ID. This avoids substring matching,
  so `frame-2d-1m` cannot also run `thermal-frame-2d-1m`.
- remote profile runs enable benchmark `--progress`, which prints per-case
  start/done lines and, for iterative SPD solves, every 256th iteration's
  residual, tolerance, and elapsed time to stderr. Start/done lines include
  the selected preconditioner and its reason while keeping stdout valid JSON
  for report files. The remote wrapper retains this stream as
  `progress.log` for both successful reports and failure receipts.
- heat-plane quad profile reports include timed memory stages, so large 400k and
  500k thermal probes can distinguish assembly, reduction, solve, and result
  scatter hotspots instead of reporting RSS-only stages
- benchmark result JSON now includes `hotspot_label`, `hotspot_elapsed_ms`,
  `hotspot_share_pct`, `hotspot_hint`, and
  `solver_preconditioner_reason`. Solver-heavy cases prefer nested
  `solve_spd_*` kernels over the outer `solve_system` wrapper, so large thermal
  and structural probes point at the actual optimization target and the chosen
  solver strategy remains auditable.
- `SOLVER_PRECONDITIONER=auto` is available for exploratory large thermal
  structural and heat-plane quad probes; it selects IC(0) for thermal-plane
  triangles and one-million-node thermal-plane quads, symmetric Gauss-Seidel
  for heat-plane and smaller thermal-plane quads, and Jacobi elsewhere
- current 500k heat-plane quad remote evidence is solver-bound: with `auto`,
  `heat-plane-quad-500k` completes in about `8.13 s` at roughly `596 MiB` peak
  RSS, with most leaf solver time under `solve_spd_preconditioner`; the next
  optimization targets are stencil-aware, multigrid, or parallel
  preconditioning plus sparse matrix-vector work
- local retained run folders are now indexed and pruned by retention count so
  nightly artifact history does not sprawl indefinitely on the runner workspace
- `400k` is exploratory, not a default nightly tier. Use narrow thermal and
  mechanical probes first, then promote only stable matrices into checked
  baselines.
- `500k` is shape-covered but lab-probe-first. `mechanical-core`,
  `thermal-core`, and `compound-core` now have retained 500k evidence on
  `kyuubiki-lab`: axial bar, truss roof, 3D space frame, triangle plane panel,
  quad plane panel, heat plane quad, compound surface panel, and the
  thermal-structural matrix all pass.
- the 500k coverage gate is now complete across `mechanical-core`,
  `thermal-core`, `compound-core`, and `thermal-structural`. Thermal truss and
  frame templates now use profile-scaled generators and have a shape-only
  regression guard, but earlier retained thermal-structural runs that used
  small fixture generators should be superseded by fresh lab pressure evidence
  before they are treated as final 500k timings.
- current 500k mechanical surface and truss evidence is solver-bound at about
  `65-67 s` for `truss-roof-500k`, `plane-panel-500k`, and
  `plane-quad-panel-500k`, with peak RSS around `1.7-2.1 GiB`. The dominant
  internal stages are sparse preconditioning and sparse matrix-vector work.
- current 1m exploratory evidence covers two simple 1D probes:
  `axial-bar-1m` completes on `kyuubiki-lab` in about `45 ms` at roughly
  `1.49 GiB` peak RSS, and `thermal-bar-1m` completes in about `505 ms` at
  roughly `2.68 GiB` peak RSS. `spring-chain-1m` now completes in about
  `553 ms` at roughly `1.23 GiB` peak RSS through the guarded tridiagonal
  chain path; arbitrary spring topologies continue to use the generic sparse
  SPD path. `torsion-shaft-1m` uses the same guarded scalar-chain route and
  completes in about `546 ms` at roughly `3.44 GiB` peak RSS. The first 1m 2D
  thermal triangle probe, `heat-plane-triangle-1m`, completes in about
  `23.17 s` at roughly `4.16 GiB` peak RSS. The first 1m 2D thermal quad
  probe,
  `heat-plane-quad-1m`, completes in about `21.0-21.3 s` at roughly `1.19 GiB`
  peak RSS with `symmetric-gauss-seidel`; the retained hotspot-aware run marks
  `solve_spd_preconditioner` at about `11.3 s`, roughly `54%` of total median
  time. The explicit IC(0) comparison reduced iterations from 1,182 to 1,122
  but took about `23.56 s` at the same peak RSS, so `auto` intentionally keeps
  symmetric Gauss-Seidel for this heat-plane family. Treat 1m as exploratory
  lab evidence, not a scheduled coverage gate yet.
- `truss-roof-1m` completes in about `134.15 s` at roughly `3.39 GiB` peak
  RSS using IC(0), after 3,072 iterations. Auto selection therefore uses IC(0)
  for one-million-node trusses while retaining symmetric Gauss-Seidel below
  that scale.
- `plane-quad-panel-1m` completes in about `165.94 s` at roughly `4.26 GiB`
  peak RSS using IC(0), after 2,816 iterations. Auto selection likewise uses
  IC(0) for one-million-node structural quad panels.
- `plane-panel-1m` completes in about `169.10 s` at roughly `4.17 GiB` peak
  RSS using IC(0). Auto selection likewise uses IC(0) for one-million-node
  structural triangle panels.
- `space-frame-1m` completes in about `2.71 s` at roughly `3.84 GiB` peak
  RSS. Its current solver path is not the generic iterative SPD path, so the
  benchmark preconditioner argument is intentionally not reported as active.
- `thermal-truss-2d-1m` completes in about `152.65 s` at roughly `4.20 GiB`
  peak RSS. Its profile path reports `ic0` with
  `auto-large-thermal-truss-ic0`, so the large-scale strategy is auditable.
- the 1M benchmark catalog has retained successful evidence for all `39/39`
  case IDs. The profile index separately reports a strict node-scale result:
  `37/39` cases have at least 1,000,000 nodes and `2/39` remain below that
  threshold. The frame and thermal-frame 2D/3D cases have independently
  completed at the full scale with IC(0) selected automatically. The nonlinear
  spring and contact-gap chains use the tridiagonal direct path at full scale.
  Spring-grid/cage, modal, and solid-tetra cases remain intentionally small
  until scalable generators are added.
- current 500k compound evidence matches the mechanical profile: the compound
  surface panel passes in about `67.7 s` at roughly `2.0 GiB` peak RSS, while
  the compound heat-plane quad passes in about `8.2 s`.
- the first `400k` probes passed for axial bar, thermal quad, truss, 3D
  space-frame, triangular structural surface, and quad structural surface
  cases, with peak RSS ranging from roughly `404 MiB` to `1.85 GiB`. Treat
  those numbers as exploratory evidence rather than hard regression baselines
  until repeat runs are available.
- `thermal-structural 400k` now has per-case progress and usable single-case
  probes. `thermal-bar-400k` uses a chain-specific fast path, and
  `thermal-plane-triangle-400k` has stage profiling plus fixed validation and
  precompute paths. Current lab evidence is about `97.33 s` with Jacobi and
  `64.78 s` with symmetric Gauss-Seidel. The matching
  `thermal-plane-quad-400k` auto probe is about `64.42 s` with roughly
  `1.59 GiB` peak RSS. A full `thermal-structural-400k` auto smoke now passes
  all nine cases in about `121.50 s` summed median time with roughly `1.59 GiB`
  peak RSS. At 500k, explicit IC(0) improves thermal-plane quad from about
  `73.39 s` and 2,544 iterations to `62.57 s` and 2,262 iterations, but raises
  peak RSS from `1.85 GiB` to `2.66 GiB`. At 1M, IC(0) completes in about
  `177.30 s` and 3,194 iterations versus SGS at `200.91 s` and 3,600
  iterations, with only about 5% more peak RSS; `auto` selects IC(0) at that
  validated node scale. Checked-baseline promotion should still wait for repeat
  runs.

## Nightly lane map

Current self-hosted nightly flows have distinct jobs:

- direct-mesh Docker nightly:
  end-to-end LAN direct-mesh regression through the Docker harness
- workflow catalog nightly:
  orchestrated composite workflow regression through the Elixir catalog path
- standard benchmark nightly:
  solver-family performance regression for the standard Rust matrix trio on the
  reference lab machine
- benchmark profile exploration:
  retained 300k/400k exploratory profile summaries for scale-tier evidence,
  indexed from `tmp/benchmark-profile/*/summary.json`. This lane appears in
  the regression catalog with `gate_scope=advisory`, so it is visible in
  reports but excluded from the enforced overall gate.

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
