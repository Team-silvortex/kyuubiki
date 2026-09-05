# Testing And CI

This document is the quick map for how Kyuubiki currently validates itself in
the `daji 3.x` line.

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
  Lightweight architecture guard for the `daji 3.x` line. It runs the
  organization audit self-test and scan, version-line checks, UI automation
  contract checks, materialization plan contract checks, material exploration
  chain contract checks, retained material research bundle and bundle-index
  contract checks, TaskIR mirror and digest contract checks, traditional
  code-coverage posture checks, dependency audits, external operator package
  preflight, external operator dynamic host smoke, docs book manifest
  validation, focused Operator TaskIR control-plane tests, and the Rust live
  operator task path.
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
  `minimum_coverage_level`, currently `qualification` for the `daji 3.x`
  physics-coverage gate.
- `make check-test-coverage-posture`
  Traditional code-coverage posture guard. It validates
  `config/test-coverage-posture.json`, writes
  `tmp/test-coverage-posture.json` and `tmp/test-coverage-posture.md`, and keeps
  line/branch/function coverage separate from module-function tensors,
  language-pack coverage, physics evidence coverage, and benchmark profile
  coverage. Until every code surface has retained artifacts and enforced
  thresholds, Kyuubiki must not claim `100%` traditional code coverage.
- `make coverage`
  Writes the same posture report without the checker self-test. This is the
  stable entrypoint for gradually wiring `cargo llvm-cov`, `c8`, `coverage.py`,
  and Elixir coverage tooling into the current test stack.
- `make coverage-rust`
  Runs the first instrumented traditional coverage lane through
  `cargo llvm-cov` and writes `tmp/coverage/rust/lcov.info` by default. Use
  `PACKAGE=<crate>` and `TEST_FILTER=<filter>` for small probes before running
  the full Rust workspace lane.
- `make coverage-frontend`
  Runs frontend unit tests through Node's built-in test coverage mode and
  writes raw V8 coverage JSON under `tmp/coverage/frontend/v8` by default. Use
  `FILTER=<domain>` for a smaller probe before running all frontend unit tests.
- `make audit-dependencies`
  Reproducible dependency security audit. It runs npm production dependency
  audits for the frontend and desktop packages, then RustSec `cargo audit` for
  the Rust workspace, Rust SDK, and every Tauri desktop shell, plus separate
  Hex retirement and OSV locked-version vulnerability checks. The Make target
  runs the audit lane self-test before invoking external tools. The checked
  `Cargo.lock`, `package-lock.json`, and `mix.lock` files under those roots are
  part of this contract.
- `make check-system-security-qualification`
  Fast, offline revalidation of the retained Daji P0 system-security report.
  It validates all 20 check identities, 40 successful round receipts, 28 exact
  module/security-lane coordinates, assertions, digests, and summary counts
  without rerunning external audit tools.
- `make qualify-system-security`
  Executes the full qualification twice and writes a fresh report. It combines
  desktop least-privilege and secret-storage assertions, control-plane auth and
  replay tests, Engine and Installer fuzz boundaries, Agent TaskIR/artifact
  admission, component integrity, and npm/Cargo/Hex dependency audits. Use this
  heavier lane for security changes and release candidates.
- `make check-persistence-provenance-qualification`
  Revalidates the retained six-module persistence/provenance report, including
  all seven suite identities, 14 repeated rounds, 28 assertions, source
  contracts, output digests, and summary counts without rerunning the suites.
- `make qualify-persistence-provenance`
  Runs desktop audit-chain, Workbench closure, Engine result-digest, Headless
  task-lineage, Orchestra recovery, and Installer journal suites twice, then
  writes a fresh machine-verifiable report.
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
- `make qualify-operator-sdk-multihost-operational-remote REMOTE=kyuubiki-lab`
- `make check-operator-sdk-multihost-operational-qualification`
- `make qualify-operator-package-acquisition-operational-remote REMOTE=kyuubiki-lab`
- `make check-operator-package-acquisition-operational-qualification`
- `make qualify-operator-sdk-performance`
- `make check-operator-sdk-performance-qualification REPORT=releases/usability-evidence/2.19.0/operator-sdk-performance-qualification.json`
- `make qualify-operator-sdk-windows-operational` on native x86_64 MSVC Windows
- `make check-operator-sdk-windows-operational-qualification`

This runs:

- Python SDK smoke tests
- Elixir SDK smoke tests
- Rust SDK smoke tests

The package-acquisition qualification is a separate two-physical-host release
lane. It starts a real Elixir Orchestra on macOS, deploys the remote Linux Agent
through Installer-owned activation, publishes one Linux operator package only
to Orchestra, and dispatches two disposable TaskIR executions. Passing evidence
requires two authenticated resolve/manifest/entrypoint download sequences,
dynamic execution, post-execution eviction, a later refetch, zero active
packages at both observation boundaries, and complete removal of temporary
credentials and managed test roots. Retained evidence never contains host
addresses, SSH aliases, usernames, credentials, or absolute host paths.
The package manifest, distribution index, resolved response, Installer receipt,
and execution report must all agree on `kyuubiki.operator-json-c/v1`. The
qualification deliberately builds the Agent workspace and operator template
with their independent dependency locks to prove that no Rust object layout or
allocator ownership crosses the dynamic-library boundary.

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
test that loads and dispatches the template operator. It then exercises direct
Agent dispatch and an Installer-managed install, Agent dispatch, tamper
rejection, recovery, uninstall, and residue-pruning journey. It writes
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

The multihost qualification target repeats the canonical six stages on native
macOS aarch64 and a physical Linux x86_64 host. It uses one unique remote work
root per run, retrieves four content-bound child reports, removes local and
remote staging, and retains only normalized evidence under
`releases/usability-evidence/2.16.4/`. Its checker includes negative self-tests
for attachment tampering and false Windows completion. Passing this target does
not promote Windows or mark the release complete.

The Windows qualification v2 lane is native rather than cross-compiled. It
binds the six-stage report to `kyuubiki.operator-json-c/v1`, verifies the ABI in
both the dynamic smoke and Installer preflight attachments, and hashes the
actual SDK, Engine, Installer, Agent CLI, template, and qualification sources.
GitHub Actions validates the report generated in the current run before
uploading it; repository-retained historical evidence is never used as the CI
pass condition.

The operator SDK performance qualification is a release-only in-process dynamic
ABI lane. It rejects empty package discovery, verifies package/operator
traceability, warms the loaded library, and records activation, first dispatch,
compact and 4096-value p50/p95/max latency, plus four-worker throughput. Its
checker binds the report to the measured sources and rejects response-copy,
latency, throughput, or dispatch-error regressions without rerunning the heavy
measurement in every architecture check.

These tests use small local loopback fixtures and focus on:

- `AgentClient.run_study`
- result fetch
- chunk browsing

### Recovery qualification checks

- `make check-linux-host-power-loss-qualification`

The retained Linux host-loss lane is a physical, two-phase qualification rather
than a simulated process restart. Its checker reruns contract and negative
self-tests, validates the SHA-256-bound reboot intent semantics, and verifies
the retained `2.19.0` report. Passing requires a changed boot identity on the
same machine, interruption of the pre-reboot Agent sentinel, unchanged
Installer-managed payload, stable solver result and TaskIR recovery behavior,
quiescent watchdog state, and zero qualification residue. The heavy capture is
run manually on the managed Linux host; ordinary CI only revalidates the
portable retained evidence.

### Cross-process integration checks

- `make test-integration`
  top-level cross-process smoke suite
- `./scripts/kyuubiki headless-live-test`
  native Rust live headless service-executor suite
- `./scripts/kyuubiki headless-rust-live-test`
  compatibility alias for the same Rust suite

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

The language-pack lane uses the native runner for full visible-copy reporting
and strict coverage. The native script audit also rejects direct
`node scripts/*.mjs` calls from Make and CI, so release checks cannot silently
fall back to a second JavaScript implementation.

Use `make check-runtime-recovery-fault-injection` for deterministic workflow
recovery testing. It injects the same unsupported condition fault once with
branch-isolation recovery and once without recovery, proving both continued
independent work and explicit fail-fast behavior without external services. It
also executes Agent watchdog failure and stale-heartbeat timeout scenarios,
including progress refresh, slot release, reason retention, late-result
deduplication, and a healthy follow-up execution.

Use `make check-orchestra-recovery-fault-injection` for the control-plane side
of recovery. It starts disposable TCP Agents and proves three post-dispatch
paths: idempotent failover, replay blocking for an unchecked side effect, and
failover after an explicit checkpoint. The native validator rejects missing
process-loss reasons, unsafe retries, duplicate side effects, or incomplete
fallback observations. The checkpointed lane must retain a verified checkpoint
digest; a caller-provided label without verification remains blocked.

Use `make check-installer-recovery-fault-injection` for deployment-side
recovery. It writes the native Installer journal through an atomic
main/next/previous store, injects process loss between commit renames, leaves a
partial next file, and verifies resume from `sync-artifacts` without replaying
the completed prefix. A second scenario proves digest tampering is rejected.
The retained report can be rechecked with
`check-installer-recovery-fault-injection --verify-report <path>` without
rerunning the probe.

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

Start `./scripts/kyuubiki start-local` from the repository root first. The
layout/search guard then exercises the native Rust frontend gateway and live
benchmark route used by packaged builds. `npm run dev` remains available only
for isolated UI hot reload; plain build validation stays fast and headless.

For service-executor and headless workflow contract changes, prefer the live
headless entrypoints before broader integration suites:

- `./scripts/kyuubiki headless-live-test`
- `./scripts/kyuubiki headless-rust-live-test`

These names boot the same temporary local control plane under
`apps/web/test/support` and
exercise real HTTP execution instead of dry-run-only fixtures.

## CI structure

CI installs the Rust channel declared in `config/toolchains.json` and
`rust-toolchain.toml`; `make check-toolchains` rejects floating or mismatched
Rust setup actions in the main and Windows qualification workflows. Rust tests
use the committed lockfile and run the engine both in the full workspace
(including unified JSON features) and independently. The workspace run collects
all failing test targets instead of stopping at the first one. SDK and
integration smoke jobs are self-contained and do not wait on unrelated unit
test jobs. Hex audit, SDK, integration, and live desktop regression explicitly
fetch their Elixir dependencies even after
a cache restore. Live GUI tests install BEAM rather than assuming another
job's environment is available.

Workflow layout preflight follows stable automation selectors through overview,
topology, contract/dataset, and diagnostic views before testing each mounted
panel. It must not depend on translated captions, the old editor layout, or
eager mounting. Historical operator qualification records are checked against
their own release snapshot; the active roadmap and evidence kits must still
agree with each other. Upgrading the product line does not relabel old evidence.

Current GitHub Actions jobs are intentionally separated:

- `web-test`
- `rust-test`
  Runs Rust formatting, workspace tests, the `800` line-count audit, and the
  medium benchmark regression gate.
- `dependency-audit`
  Should run `make audit-dependencies` when dependency or lockfile surfaces
  change, and before release branches are cut.
- `frontend-test`
- `architecture-contracts`
- `workflow-preflight`
- `desktop-gui-regression`
  Runs the complete UI invocation gate with a real local runtime and retains
  failure screenshots, DOM state, and runtime logs.
- `sdk-smoke`
- `integration-smoke-api`
- `integration-smoke-cluster`
- `integration-smoke-direct-mesh`
- `desktop-gui-smoke-hub`
- `desktop-gui-smoke-installer`
- `desktop-gui-smoke-workbench`

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
- `cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix dynamic-response --repeat 1`
  Run isolated transient-heat, transient-spring, and harmonic-spring Engine
  probes without promoting the experimental dynamic lane into qualification.
- `cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix cohesive-interface --repeat 1`
  Run the 1D/2D constitutive and 2D/3D assembled cohesive-interface paths
  through isolated native Engine processes. History workloads cap at 4096
  steps; mesh workloads retain their actual bounded 512-node contract rather
  than inheriting a misleading profile-scale node claim. JSON reports expose
  the executed count as `history_step_count`.
- `cd workers/rust && cargo run -q -p kyuubiki-script-runner -- check-operator-validation --execute --profile electric-conduction-plane-quad-screening`
  Execute the electric-conduction component dossier: rotated closed form, mesh
  refinement, malformed topology, Agent RPC, Engine Workflow, and Rust
  headless-contract checks. Passing this profile does not add the operator to
  the release-qualified `physics-coverage` matrix.
- `cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile medium --matrix thermal-structural --repeat 1`
  Run the coupled thermal-structural smoke matrix for thermal bar/truss/plane,
  static frame, and thermal frame families.
- `make benchmark-physics-coverage`
  Run the `daji 3.x` broad physics smoke matrix across every built-in benchmark
  template. This is the quickest product-level check that the main physics
  families still have real solver execution paths while engine and TaskIR
  contracts harden.
- `cd workers/rust && cargo test -p kyuubiki-engine workflow_large_graphs --release --lib -- --nocapture`
  Exercise 128/256/512/1024-layer real operator chains through the compiled
  workflow execution plan. The 1024-layer lanes cover fully reversed node
  declarations, `Ephemeral` pass-through artifacts, and output-only result
  projection. The ephemeral lane must retain only the five non-temporary chain
  artifacts; the projected lane must retain only `thermo_output.result`. Both
  preserve completion and lineage. Timing is host-local diagnostic data;
  ordering, output-budget checks, retained artifact counts, outputs, and lineage
  are regression assertions.
- `cd workers/rust && cargo test -p kyuubiki-engine workflow_scheduler_hot_path --release --lib -- --nocapture --test-threads=1`
  Isolate 1024-layer scheduler, trace, progress, and artifact-retention overhead
  without solver work. Cached and ephemeral lanes assert identical output and
  lineage while retaining 1026 and 2 artifacts respectively. Treat elapsed time
  as host-local diagnostic evidence rather than a release threshold.
- `cd workers/rust && cargo test -p kyuubiki-engine workflow_solver_hot_path --release --lib -- --nocapture --test-threads=1`
  Split the real heat-to-thermo path into transform-registry lookup, heat solve,
  bridge API/core, thermo-mechanical solve, and 1024-value materialization.
  This lane also runs concurrent bridges against the shared immutable built-in
  registry. Compare segment proportions between revisions; do not promote one
  host's microsecond values into cross-platform release thresholds.
- `make benchmark-standard-nightly`
  Sync the Rust workspace without `target/` plus its root `schemas/` compile-time
  fixtures to `kyuubiki-lab`, run the standard regression trio there, and pull
  the resulting reports back under `tmp/standard-benchmark/`. Remote profile
  sync follows the same self-contained source contract, so CLI test binaries do
  not depend on a pre-existing full repository checkout.
- `make benchmark-profile-remote PROFILE=medium MATRIX=material-integration REPEAT=3`
  Compare fixed two-point and adaptive 2/3/4/8/12-point frame material
  integration on the Linux lab host. The retained summary reports timing and
  response deltas, and the runner fails if the pair is incomplete or changes
  Newton iterations, residual, maximum displacement, or maximum stress beyond
  its numerical equivalence tolerance.
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
  `./scripts/kyuubiki build-benchmark-profile-index --coverage-targets <manifest>`
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
  all `39/39` cases have at least 1,000,000 nodes with no remaining below-threshold cases.
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

## Operational Orchestra Takeover Qualification

The active-owner lane runs two independent local Orchestra BEAM processes
against one ephemeral PostgreSQL instance on the remote Linux qualification
host:

```sh
make qualify-orchestra-takeover-operational-remote REMOTE=kyuubiki-lab
make check-orchestra-takeover-operational-qualification
```

The capture keeps PostgreSQL on the remote loopback interface and reaches it
through a temporary SSH loopback tunnel. It proves owner election, standby
exclusion, forced owner process loss, fencing-token increment, standby
promotion, and former-owner rejoin fencing. Success additionally requires
removal of both BEAM processes, all local ports and logs, the tunnel, and the
remote container. The retained report contains roles and timings only, never a
host address, account, database URL, or credential.

The database-partition lane keeps both Orchestra processes alive and gives each
one an independent SSH loopback tunnel to the same PostgreSQL instance:

```sh
make qualify-orchestra-network-partition-operational-remote REMOTE=kyuubiki-lab
make check-orchestra-network-partition-operational-qualification
```

It removes only the current owner's tunnel, requires that owner to demote with
`orchestra_lease_store_unavailable`, proves the standby path remains available,
then observes a higher fencing token. After restoring the old owner's network,
the lane requires it to rejoin as standby and rejects a valid workflow write
with `orchestra_standby`. The lease-only introspection route avoids depending
on unrelated database-backed health components during the partition.

The long-workflow lane pauses an exact remote Agent execution while two local
Orchestra processes share the remote PostgreSQL recovery store:

```sh
make qualify-orchestra-long-workflow-takeover-operational-remote REMOTE=kyuubiki-lab
make check-orchestra-long-workflow-takeover-operational-qualification
```

It proves that an idempotent workflow advances from generation one to two,
dispatches exactly twice, and produces one verified terminal commit. A separate
`checkpoint_required` workflow stays on generation one, enters
`recovery_blocked`, is not redispatched, and cannot be mutated by the orphaned
completion. Both former owners must rejoin as standby, a follow-up solve must
succeed, and all Agent, Orchestra, tunnel, database, port, and work-root state
must be removed. The retained report is host-identity and credential free.

This is source-runtime operational evidence. The separate installed-package
lane builds and activates a production OTP release on the remote Linux host,
deletes the synchronized source tree, and then repeats the same takeover:

```sh
make qualify-orchestra-installed-takeover-operational-remote REMOTE=kyuubiki-lab
make check-orchestra-installed-takeover-operational-qualification
```

Both installed Orchestra instances share one digest-verified immutable Runtime
payload while keeping independent writable release state. Success requires an
Installer activation record, source fallback disabled, owner crash and token
increment, former-owner fencing, plus zero managed Runtime, container, process,
port, run-root, or transient evidence residue. Fleet acquisition and non-Linux
packages remain separate open qualifications.

The installed end-to-end Runtime lane exercises the ordinary standalone path,
including the installed Rust Headless client and two installed Rust Agents:

```sh
make qualify-installed-runtime-operational-remote
make check-installed-runtime-operational-qualification
make qualify-installed-runtime-operational-macos
make check-installed-runtime-macos-operational-qualification
```

It builds one production OTP release and the native binaries, seals and
activates them through Installer, deletes the synchronized source tree, then
starts the Runtime with the frontend disabled. Headless submits a real bar
solve and fetches the same persisted result after each of two managed Runtime
restarts. Before release assembly, every production Elixir module is compiled
with `mix compile --warnings-as-errors` on the pinned physical-Linux toolchain;
type-analysis or compiler warnings therefore fail the qualification rather
than being hidden in release output. Installed state paths are absolute and
outside the immutable payload; packaged relative development defaults cannot
redirect SQLite, JSON persistence, or artifacts into installed program files.
The host capture verifies the complete payload file set both before startup and
after shutdown, and fails closed on digest drift, source fallback, changed
numerical output, missing Agent dispatch, stale PID files, open ports, or
cleanup residue. Only the compact path-free report is retained; the remote run
root and local raw captures are removed.

The macOS command drives the same contract locally on Apple Silicon without
using the source tree as a runtime fallback. It stages, seals, installs, and
activates the payload under a temporary macOS `HOME`, runs the full installed
Headless-Orchestra-two-Agent-Engine chain through two restarts, rescans the
immutable payload, and removes both the application-support store and raw
captures. This is ordinary installed-operation evidence, not a substitute for
a physical macOS reboot qualification.

The full installed Runtime reboot lane is a separate two-phase boundary. It
persists one completed Headless job while the installed Orchestra and two Rust
Agents are still running, then requires a physical Linux reboot before the same
job can be fetched again:

```sh
make qualify-installed-runtime-power-loss-remote ACTION=prepare
make qualify-installed-runtime-power-loss-remote ACTION=reboot ARGS=--confirm-physical-reboot
make qualify-installed-runtime-power-loss-remote ACTION=resume
make check-installed-runtime-power-loss-qualification \
  REPORT=releases/usability-evidence/2.19.0/installed-runtime-power-loss-operational-qualification.json
```

The native command itself requires `--confirm-physical-reboot`; the Make target
does not bypass that guard. An abandoned run is removed with `ACTION=cleanup`.
Preparation and resume use a digest-bound local session plus a durable remote
intent, verify every file named by the sealed Runtime payload, keep the source
tree detached, and reject a resume unless the machine identity is unchanged and
the boot identity changed. Payload integrity is checked after pre-reboot work,
before recovery, and again after the recovered Runtime stops. The retained
physical-Linux report passes 16/16 checks, including exact persisted-job and
numerical-result continuity, released ports, removed PID state, and zero remote
residue. Only the final path-free report is retained; this evidence does not
claim installed recovery on macOS or Windows.

## Operational Agent Solver Qualification

The operational solver lane closes the gap between a source-level TaskIR test
and an Installer-managed release binary on a separate Linux host:

```sh
make qualify-agent-solver-operational-remote REMOTE=kyuubiki-lab
make check-agent-solver-operational-qualification
```

The capture command syncs only the Rust workspace into an isolated lab-run
root, reuses a server-side Cargo target cache, builds release Agent and
Installer binaries, and delegates package sealing, activation, execution, and
qualification cleanup to the native Installer. SSH remains transport only; it
does not replace Installer deployment semantics.

Each accepted report must prove:

- a digest-verified `kyuubiki.agent-update-package/v1` activation
- two distinct Agent process instances executing the same TaskIR digest
- zero-error closed-form axial-bar results before and after each failure set
- unsupported-solver and TaskIR-digest rejection in both process instances
- watchdog quiescence and successful recovery in both process instances
- removal of the isolated qualification root with zero retained residue
- absence of host addresses, account names, credentials, and absolute host paths

The small JSON report is retained locally and under the server's managed lab
evidence root. Every successfully prepared run root is removed after the
attempt, including failed qualification or report-transfer attempts; large
release artifacts stay in the server-side Cargo cache instead of the MacBook
workspace.

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
