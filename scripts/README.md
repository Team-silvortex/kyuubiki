# Scripts

This directory contains host-native operational entry points.

- `kyuubiki`
  Thin compatibility shim for the native Rust `kyuubiki-script-runner`
  binary. New operational command logic should land in
  `workers/rust/crates/script-runner`, not in shell.
- `kyuubiki-lab`
  Thin operational wrapper for the shared Ubuntu lab machine that now hosts
  the standard download/deploy server plus the shared solver-agent test node.
- `create-release-snapshot.mjs`
  Scaffold a new lightweight release snapshot manifest and update the release
  index. When a snapshot is marked `current`, it also advances the shared
  shipping-version contracts.
- `build-update-catalog.mjs`
  Generate the shared update catalog JSON plus HTML docs from release snapshots
  and the human-owned channel contract.
- `build-installation-integrity-docs.mjs`
  Generate the installation integrity HTML docs for both the repository-level
  book and the Hub-facing mirror shelf.
- `upload-desktop-release-remote.sh`
  Upload generated desktop bundles, staged `dist/` outputs, and release
  metadata to a remote download server, with an optional post-upload local
  cleanup path for disk-constrained workstations.
- `audit-version-line.mjs`
  Audit repository-wide version contracts and inventory visible version
  references before advancing a shipping line such as `tamamono 1.7.0`. The
  exact-contract lane includes release metadata, package metadata, generated
  docs mirrors, update catalogs, shipped language-pack catalog versions, and
  hand-maintained Markdown facts such as `current-line.md` and
  `version-line.md`.
- `audit-rust-line-counts.mjs`
  Enforce the Rust source line-count ceiling, currently `600` lines per file,
  so crate and test modules stay split before they become hard to review.
- `audit-project-organization.mjs`
  Enforce the repository-wide source organization guard. New source and docs
  files, including untracked files that are not ignored, stay under the shared
  line ceiling, while explicitly tracked historical debt files are allowed only
  up to their current debt limit. It also keeps installer `tests.rs` as a
  module index instead of a growing test bucket. Use `--self-test` to verify
  the audit helper rules themselves.
- `audit-dependencies.mjs`
  Run the security dependency audit lanes that require lockfiles: npm
  production audits for frontend/desktop packages plus RustSec audits for the
  Rust workspace, Rust SDK, and Tauri desktop shells. Use `--self-test` when
  changing lane coverage or audit arguments. Lane coverage is read from
  `config/dependency-audit-lockfiles.json`.
- `make architecture-check`
  Lightweight guard for the current architecture organization line. It combines
  the repository organization audit self-test and scan, dependency audits,
  external operator package preflight, docs manifest JSON validation, focused
  Operator TaskIR API tests, and the Rust headless live operator task test.
- `check-doc-book.mjs`
  Verify the centralized docs book and Hub mirrors for version alignment,
  broken local links, required chapter markers, and old legacy wording.
- `check-elixir-self-host.mjs`
  Verify the Elixir/Mix/OTP runtime plus the orchestrator self-host
  environment contract before a machine is treated as installer-managed.
- `check-make-modules.mjs`
  Verify that the root `Makefile` stays as a small include-based entrypoint,
  that every `make/*.mk` module is included, and that retired catch-all modules
  do not come back.
- `validate-language-packs.mjs`
  Validate the shipped Workbench/Hub language support pack catalog and JSON
  envelopes for the current release index version in the `tamamono 1.x` line.
- `check-ui-automation-contract.mjs`
  Verify the product-owned Workbench automation selector contract. It compares
  `docs/ui-automation-contract.json`, the frontend TS selector constants, and
  the component implementation anchors used by wasm-python and UI smoke tests.
  Use `--self-test` when changing selector coverage.
- `check-operator-task-ir-contract.mjs`
  Verify the language-neutral TaskIR schema extension and shipped TaskIR
  examples. It checks that mirrored fields such as operator kind, package ref,
  and package version stay consistent across descriptor, execution program, and
  runtime hints, and recomputes canonical `descriptor_digest` and `task_digest`
  values, including a fractional-number fixture, before agent or SDK tests need
  to run.
- `check-materialization-plan-contract.mjs`
  Verify the shared material candidate materialization plan schema, fixture,
  and SDK documentation links. It keeps reviewed agent/lab materialization
  output aligned with the solver-rerun runner contract before SDK parity work
  consumes the same artifact.
- `check-material-study-execution-plan-contract.mjs`
  Verify the shared non-executing material study execution plan schema,
  fixture, and SDK documentation links. It keeps `--plan-study` output aligned
  with headless SDK and remote scheduler expectations before solver dispatch.
- `build-material-research-bundle.mjs`, `check-material-research-bundle.mjs`,
  `build-material-research-bundle-index.mjs`, and
  `check-material-research-bundle-contract.mjs`
  Build and verify the first retained material research bundle. The bundle
  captures initial exploration, next-round execution planning, a rerun,
  chained rounds, artifact checksums, and repo-relative reproduction commands
  under `kyuubiki.material-research-bundle/v1`. Override `STUDY=` through Make
  to build the heat-spreader or composite thermo-electric panel retained
  profile. The bundle index builder writes a compact multi-study overview under
  `tmp/material-research-bundles/` for CI, agents, and release notes. The
  lightweight contract check keeps
  `schemas/material-research-bundle.schema.json`,
  `schemas/examples.material-research-bundle.json`, and documentation links in
  sync without running the solver.
- `operator-reliability-*.mjs` and `check-operator-reliability*.mjs`
  Operator reliability gate family. `operator-reliability-contracts.mjs`
  centralizes config/schema paths and schema versions,
  `operator-reliability-rules.mjs` owns pure trust-level and qualification
  rules, `test-operator-reliability-rules.mjs` tests those rules,
  `check-operator-reliability-schemas.mjs` runs the zero-dependency schema
  smoke, and `check-operator-reliability.mjs` performs the manifest,
  benchmark, workflow payload, evidence-file, roadmap, and evidence-kit gate.
- `build-remote-material-benchmark-summary.mjs` and helpers
  Retained lab benchmark evidence summarizer. The builder reads
  `tmp/remote-material-research/`, emits JSON, and delegates Markdown rendering
  to `remote-material-benchmark-markdown.mjs`; stage summaries, optimization
  targets, sparse matvec throughput, preconditioner economics, and tuning notes
  live in `remote-material-benchmark-analysis.mjs`. Self-test fixtures live in
  `remote-material-benchmark-summary-self-test.mjs` so the builder stays small
  and import-safe. `check-remote-material-preconditioner-health.mjs` applies a
  conservative retained-evidence gate so SGS/Jacobi comparison wins cannot
  silently regress, while `check-remote-material-stage-health.mjs` verifies
  retained stage timing/share and summary fields; the generated stage summary
  tables are the optimization triage entrypoint for system-wide solver
  hotspots. Sparse matvec summaries also normalize measured rows by
  `ms / M nnz-visits`, using only samples that expose solver matrix nnz. Use
  their `--self-test` lanes when changing threshold logic. The preconditioner
  economics table reports the extra sweep cost versus the non-preconditioner
  time saved, so SGS-style changes can be judged by net solver value.
- `check-line-field-closed-form-baseline.mjs`
  Verify the first versioned qualification evidence artifact for the
  `line-field-closed-form` candidate. It checks that all four 1D closed-form
  operator baselines are present, finite, tolerance-bearing, and linked to the
  Rust accuracy-baseline tests. It also checks the candidate tolerance policy
  so tight closed-form tolerances cannot silently expand into broader claims.
- `capture-line-field-qualification-provenance.mjs`
  Emit a repo-relative provenance JSON envelope for the same candidate. The
  output records revision state, toolchain versions, platform metadata, command
  contracts, and hashes of the evidence inputs without embedding local absolute
  paths.
- `capture-line-field-qualification-release-evidence.mjs`
  Run the line-field evidence checker and Rust solver baseline, then retain
  command status, duration, output, and provenance in a repo-local JSON bundle.
  It is intended for release artifacts under `tmp/` or a release staging area,
  not for direct Git commits.
- `check-line-field-qualification-release-evidence.mjs`
  Validate a retained line-field release evidence bundle. It checks schema
  version, command success, provenance inputs, SHA-256 shape, release-retention
  flags, and absence of local absolute repository paths.
- `build-operator-qualification-readiness.mjs`
  Build a repo-local JSON readiness report for all qualification roadmap
  candidates, including artifact state, graduation gates, and a sorted
  `next_actions` queue for the highest-priority evidence collection steps.
- `check-operator-qualification-readiness.mjs`
  Validate the generated readiness report and its `next_actions` queue. Use
  `--self-test` when changing readiness sorting or action-kind requirements.
- `validate-commercial-readiness.mjs`
  Verify the `2.0` commercial-readiness manifest against its Markdown gate,
  including gate count, evidence links, and the shared exit statement.
- `validate-minimal-industrial-closure.mjs`
  Verify the narrower `1.15.x -> 1.20.x` minimum industrial closure manifest
  against its Markdown gate, including gate count, evidence links, supported
  state values, and the shared exit statement.
- `sync-doc-book-version.mjs`
  Update the hand-maintained book entry pages to the current shipping version,
  shipping-version chip, current-prep chip, and book manifest shipping version
  without touching the generated installation or update-catalog pages.
- `release-metadata.mjs`
  Shared release-path, JSON, artifact, and shipping-version helpers used by the
  release and installation-doc generators.

Use this directory for operator-facing workflow wrappers, not for source
libraries or generated output.

Shell migration rule:

- Keep `scripts/kyuubiki` as a tiny launcher only.
- Prefer Rust native commands for new cross-platform operations.
- Use `./scripts/kyuubiki native-script-audit` to list remaining shell wrappers.
- Treat the audit's `host tool boundary` section as the self-hosting backlog:
  those tools should eventually be provided by installer-managed runtimes,
  narrow native binaries, or explicit remote services rather than ad hoc shell
  assumptions.
- Keep remote transfer and command execution behind
  `workers/rust/crates/script-runner/src/remote_host.rs` so the project can
  replace host `ssh`/`scp`/`rsync` with an installer-managed or embedded
  implementation without touching every benchmark runner.
- Existing `.sh` files are migration targets unless they are generated,
  third-party, or platform package payloads.

Typical responsibilities:

- start/stop/restart orchestration
- hot-reload/watch orchestration for local development
- mode switching (`local`, `cloud`, `distributed`)
- verification/test wrappers
- component-scoped build entry points
- runtime and desktop packaging entry points
- installer entry points
- release snapshot scaffolding
- unified update-catalog generation
- release metadata normalization across `releases/` and `deploy/`
- remote release artifact upload and local bundle cleanup

Useful checks:

- `make audit-dependencies`
  Run npm `--omit=dev --package-lock-only` audits for the shipped JS app
  lockfiles and RustSec `cargo audit` against every checked Rust/Tauri
  `Cargo.lock`. The Make target runs the dependency-audit self-test first.
  Keep the lockfiles tracked for these lanes so security results are
  reproducible. Add or remove lanes through
  `config/dependency-audit-lockfiles.json`.
- `./scripts/kyuubiki rust-line-audit`
  Enforce the Rust source file line-count ceiling without running the full
  Rust test suite.
- `make check-ui-automation-contract`
  Check that product-owned Workbench UI automation anchors still match the
  documented selector contract. Run this before changing rail, library,
  runtime, viewport, control-window, or shell DOM structure.
- `make check-version-line`
  Run the version-line checker self-test, then check that release metadata,
  package metadata, generated docs mirrors, update catalogs, shipped
  language-pack catalog entries, and hand-maintained version-line docs all
  match the current shipping version.
- `make check-operator-reliability`
  Check that every `physics-coverage` solve operator has a reliability manifest
  entry with benchmark, headless workflow, evidence, visible limitations, and a
  coverage level that satisfies the release minimum gate. The Make target runs
  the focused rule, schema smoke, and line-field closed-form baseline checks
  first.
- `make check-operator-reliability-rules`
  Run only the pure reliability rule self-test without loading benchmark
  catalogs, workflow payloads, manifest shards, or evidence files.
- `make remote-material-research-summary`
  Run the remote material benchmark summary self-test, then summarize retained
  lab evidence under `tmp/remote-material-research/`. Use this after targeted
  remote reruns so single-case evidence updates do not hide the rest of the
  latest benchmark matrix. The target also checks retained preconditioner and
  stage health using the generated summary, after first running each health
  checker self-test.
- `make check-operator-reliability-schemas`
  Run only the operator reliability schema/config version smoke without loading
  benchmark catalogs, workflow payloads, or evidence files. This covers
  schema-version alignment and required-field presence, not full JSON Schema
  validation.
- `make check-materialization-plan-contract`
  Run the zero-dependency materialized candidate plan contract check and its
  self-test. Use this after changing
  `schemas/material-candidate-materialization-plan.schema.json`,
  `schemas/examples.material-candidate-materialization-plan.json`, or the SDK
  materialization documentation.
- `make check-material-exploration-chain-contract`
  Run the zero-dependency material exploration chain contract check and its
  self-test. Use this after changing
  `schemas/material-exploration-chain.schema.json`,
  `schemas/examples.material-exploration-chain.json`, chain convergence
  fields, optimization trace fields, or SDK chain documentation.
- `make check-material-research-bundle-contract`
  Run the zero-dependency retained material research bundle contract check and
  its self-test. Use this after changing
  `schemas/material-research-bundle.schema.json`,
  `schemas/examples.material-research-bundle.json`, or bundle documentation.
- `make material-research-bundle-index`
  Build the retained heat-spreader and composite thermo-electric panel bundles,
  validate them, and write `tmp/material-research-bundles/index.json` plus a
  human-readable `README.md` summary.
- `make build-operator-qualification-readiness`
  Write and validate a readiness report for the qualification roadmap. Override
  `OUT=tmp/name.json`; the report is a local planning artifact and should stay
  out of Git unless deliberately retained with a release.
- `make capture-line-field-qualification-provenance`
  Write a release-retainable provenance JSON envelope for the first
  qualification candidate. Override `OUT=tmp/name.json`; the output path must
  stay repo-local and should normally remain outside Git until attached to a
  release.
- `make capture-line-field-qualification-release-evidence`
  Run the first qualification candidate's release-retained regression evidence
  lane and write the resulting JSON bundle. Override `OUT=tmp/name.json`; keep
  routine run output outside Git and attach it to the release record instead.
- `make check-line-field-qualification-release-evidence`
  Validate the generated or staged line-field release evidence bundle. Override
  `IN=tmp/name.json` when checking a release-staging copy.
- `make check-elixir-self-host`
  Check the current machine's Elixir, Mix, OTP, and orchestrator environment
  contract against `config/toolchains.json`. Use `node
  ./scripts/check-elixir-self-host.mjs --static-only --json` when preparing an
  installer image where Elixir is not yet installed.

Useful smoke wrappers:

- `./scripts/kyuubiki smoke`
  Current Elixir -> Rust integration smoke flow.
- `./scripts/kyuubiki sdk-smoke`
  Python / Elixir / Rust headless SDK smoke suite.
- `./scripts/kyuubiki agent-capability-smoke --host 192.168.1.12 --port 5001 --output tmp/agent-capability-smoke-5001.json`
  Probe a running solver agent, read its advertised RPC methods, and run the
  matching minimal Python SDK solver fixtures. This is the preferred quick
  check for installer-managed lab agents because it reports both tested and
  untested advertised methods without mutating the remote service.
- `AGENT_HOST=192.168.1.12 AGENT_PORT=5001 AGENT_SMOKE_PROFILE=lab-legacy-26 make test-agent-capability-smoke`
  Run the same check through Make with an explicit release gate. Raise
  `AGENT_SMOKE_PROFILE` to `current-40` for a local `1.16.x` agent with the
  newer dynamic, acoustic, magnetic, fluid, and solid solver RPC surface. Use
  `AGENT_SMOKE_ARGS="--expect-kind solid_tetra_3d"` for additional one-off
  release assertions.
- `cargo run -p kyuubiki-installer -- embedded-runtimes`
  Print the installer-managed runtime payload contract for the current
  platform. The same data is written to
  `dist/<platform>/manifests/embedded-runtimes.json` during `stage-release`.
- `KYUUBIKI_RUNTIME_STRICT=1 ./scripts/kyuubiki-runtime.mjs status`
  Resolve runtime commands from the embedded runtime manifest and fail if a
  required self-host runtime payload is missing instead of silently using the
  host PATH.
- `./scripts/kyuubiki frontend-test`
  Frontend typecheck plus production build verification.
- `./scripts/kyuubiki headless-test`
  Frontend-owned headless CLI regression suite covering template selection,
  workflow export, validation, dry-run execution, and risk gating.
- `./scripts/kyuubiki headless-live-test`
  Live headless smoke that boots a temporary local control plane with fake
  solver sessions, then drives real `headless run --execute` workflow jobs
  through the service executor over HTTP. The current suite covers
  `service_health`, catalog-backed workflow submission, and inline
  `workflow_submit_graph` submission with explicit agent-failure surfacing.
- `./scripts/kyuubiki headless-rust-live-test`
  Rust `kyuubiki-headless` live integration suite against the same temporary
  local control plane, covering service-health, catalog-workflow execution,
  and inline `workflow_submit_graph` execution through the Rust service
  executor.
- `./scripts/kyuubiki workflow-preflight`
  Workflow topology plus search/layout guard suite. Start `npm run dev` under
  `apps/frontend` in a separate shell first because the browser-backed checks
  exercise the live workbench benchmark surface.
- `./scripts/kyuubiki operator-package-preflight ./operator-packages`
  Read-only external operator package admission report. It prints
  `kyuubiki.operator-package-preflight/v1` JSON with accepted packages,
  rejected package reasons, and a safety block confirming that dynamic
  libraries were not loaded.
- `make operator-package-preflight`
  Runs the same admission report against the checked-in Rust operator crate
  template under `workers/rust/templates/`.
- `make operator-package-preflight OUT=tmp/operator-package-preflight.json`
  Writes the same report to a repo-root-relative JSON file for CI artifacts or
  installer diagnostics.
- `make operator-package-preflight FAIL_ON_REJECTED=1`
  Turns rejected external operator packages into a non-zero quality gate.
- `./scripts/kyuubiki desktop-upload-remote macos`
  Upload the current shipping-version desktop release outputs to the remote
  download server. Override the target with
  `KYUUBIKI_RELEASE_REMOTE_HOST=user@host`. Prefer SSH keys or an agent; the
  temporary `KYUUBIKI_RELEASE_REMOTE_PASSWORD=...` compatibility path uses
  `sshpass -e`, and `PURGE_LOCAL=1` removes local `dist/` and platform-matched
  Tauri bundle outputs after a successful upload.
- `./scripts/run-direct-mesh-benchmark-container.sh --repeat 3`
  Compatibility shim for
  `./scripts/kyuubiki direct-mesh-benchmark-container --repeat 3`. It builds
  the dedicated Docker harness, runs the direct-mesh integration suite multiple
  times, and writes JSON plus Markdown summaries under
  `tmp/direct-mesh-benchmark-container/`. For LAN agent discovery, prefer
  `DOCKER_RUN_NETWORK=host`. The current checked-in baseline snapshot is
  `tests/integration/benchmarks/direct-mesh-docker-baseline.json`.
- `node ./scripts/compare-direct-mesh-benchmark.mjs --current tmp/direct-mesh-benchmark-container/latest/summary.json --baseline tests/integration/benchmarks/direct-mesh-docker-baseline.json --report-out tmp/direct-mesh-benchmark-container/latest/compare.md --json-out tmp/direct-mesh-benchmark-container/latest/compare.json`
  Compare a direct-mesh Docker benchmark summary against the checked-in
  baseline and emit both Markdown and machine-readable diff artifacts.
- `./scripts/run-direct-mesh-benchmark-regression.sh`
  Compatibility shim for
  `./scripts/kyuubiki direct-mesh-benchmark-regression`. It runs the remote
  direct-mesh Docker benchmark on `kyuubiki-lab`, copies the resulting summary
  back into the local workspace, and compares it against the checked-in
  baseline with regression thresholds. This native command expects a narrow
  passwordless `sudo` rule for the benchmark wrapper on the remote lab host.
- `cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_large_graph_report_test.exs`
  Runs the orchestrated large-graph workflow benchmark suite and writes a
  machine-readable JSON report with per-case performance summaries at
  `../tmp/workflow-large-graph-benchmark.json` from `apps/web`, which is the
  repository-level `tmp/workflow-large-graph-benchmark.json`.
- `cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs`
  Runs the catalog-backed composite workflow benchmark suite for the current
  thermal and guarded coupled flows across the default 8-case quad/triangle
  suite, then writes a machine-readable JSON report at
  `../tmp/workflow-catalog-benchmark.json` from `apps/web`, which is the
  repository-level `tmp/workflow-catalog-benchmark.json`. The current checked-in
  baseline snapshot is
  `tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json`.
- `node ./scripts/compare-workflow-catalog-benchmark.mjs --current tmp/workflow-catalog-benchmark.json --baseline tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json --report-out tmp/workflow-catalog-benchmark.compare.md --json-out tmp/workflow-catalog-benchmark.compare.json`
  Compare a workflow catalog benchmark report against the checked-in 8-case
  baseline and emit both Markdown and machine-readable diff artifacts.
- `make test-integration-workflow-catalog-compare CURRENT=tmp/workflow-catalog-benchmark.json`
  Makefile entry for comparing a workflow catalog benchmark report against the
  checked-in 8-case baseline.
- `make test-integration-workflow-catalog-report`
  Run the local workflow catalog benchmark report case and compare it against
  the checked-in baseline.
- `./scripts/run-workflow-catalog-benchmark-regression.sh`
  Compatibility shim for
  `./scripts/kyuubiki workflow-catalog-benchmark-regression`. It runs the
  remote workflow catalog benchmark on `kyuubiki-lab`, copies the resulting
  summary back into the local workspace, and compares it against the checked-in
  baseline with per-case regression thresholds.
- `./scripts/kyuubiki workflow-mesh-regression`
  Native runner command for the local workflow mesh regression. It runs the current
  three-test distributed workflow mesh regression trio in strict sequence on
  the local machine so the shared local orchestrator port does not collide
  across tests. This emits `run.log`, `summary.json`, and `README.md` under
  `tmp/workflow-mesh-regression/<slug>/`.
- `./scripts/run-workflow-mesh-regression.sh`
  Compatibility shim for the native command above. Prefer the native command
  or `make test-integration-workflow-mesh` for new automation.
- `./scripts/run-workflow-mesh-regression-remote.sh`
  Compatibility shim for
  `./scripts/kyuubiki workflow-mesh-regression-remote`. It syncs the mesh
  workflow regression wrappers plus integration tests to `kyuubiki-lab`, runs
  the distributed workflow mesh regression trio there, and pulls the combined
  run log plus summary artifacts back into `tmp/workflow-mesh-regression/`.
- `./scripts/build-workflow-mesh-regression-summary.mjs --log tmp/workflow-mesh-regression/<slug>/run.log --output-dir tmp/workflow-mesh-regression/<slug>`
  Rebuild the machine-readable and human-readable summary artifacts for a
  workflow mesh regression run from the captured TAP log.
- `./scripts/build-workflow-mesh-regression-index.mjs --root tmp/workflow-mesh-regression`
  Rebuild the retained workflow mesh run index and emit `index.json`,
  `README.md`, and `index.html` under `tmp/workflow-mesh-regression/`.
- `make test-integration-workflow-mesh`
  Makefile entry for the local distributed workflow mesh regression trio. This
  delegates directly to the native runner instead of invoking the shell shim.
- `make test-integration-workflow-mesh-nightly`
  Makefile entry for the remote `kyuubiki-lab` workflow mesh regression flow.
- `make test-integration-workflow-catalog-nightly`
  Makefile entry for the remote workflow catalog regression flow against the
  checked-in baseline.
- `PROFILE=400k MATRIX=mechanical-core CASE=axial-bar-400k REPEAT=1 ./scripts/run-benchmark-profile-remote.sh`
  Compatibility shim for `./scripts/kyuubiki benchmark-profile-remote`. It runs
  one remote Rust benchmark profile/matrix smoke without requiring a checked
  baseline. Use this for new scale tiers before promoting them into the
  standard regression gate. Outputs land under `tmp/benchmark-profile/` as the
  raw benchmark JSON, `README.md`, and a compact `summary.json`.
  The remote profile runner defaults to `SOLVER_PRECONDITIONER=auto`, which
  keeps Jacobi for general cases but selects symmetric Gauss-Seidel for
  thermal plane triangle/quad workloads. Set `SOLVER_PRECONDITIONER=all` on
  solver probes to emit Jacobi and symmetric Gauss-Seidel rows in the same
  smoke report. Set `REPORT_ONLY=1` with the same `PROFILE`, `MATRIX`, `CASE`,
  and `OUTPUT_SLUG`/`LOCAL_OUTPUT_DIR` when regenerating Markdown from an
  already copied JSON report without SSH, rsync, or rerunning a large case.
  Set `LOCAL_JSON_PATH=/absolute/path/to/report.json` when backfilling
  summaries from older, non-standard profile JSON filenames.
- `./scripts/build-benchmark-profile-index.mjs`
  Rebuild the exploratory benchmark profile run index under
  `tmp/benchmark-profile/` from retained `summary.json` files. This emits
  `index.json` plus `README.md` without rerunning any benchmark. Its gate is
  advisory and checks only retained-run presence plus finite case/time/RSS
  metrics. Malformed retained `summary.json` files are recorded under
  `skipped_runs` and do not abort index generation. The index also emits
  `matrix_summaries` so mechanical, thermal, and coupled exploratory evidence
  can be reviewed by matrix instead of by individual run folder only. It also
  emits `coverage_summaries` for release-scale matrix completeness checks such
  as `mechanical-core` `400k`. New profile summaries should provide
  `case_ids`; the index falls back to `slowest_case` only for old summaries.
  Coverage targets are loaded from `config/benchmark-profile-coverage.json` by
  default and can be overridden with `--coverage-targets <manifest.json>`.
  The coverage manifest is validated strictly so malformed targets fail before
  a misleading empty coverage report can be generated.
- `./scripts/run-standard-benchmark-regression.sh`
  Compatibility shim for `./scripts/kyuubiki standard-benchmark-regression`.
  It syncs the Rust workspace without `target/` to `kyuubiki-lab`, runs the standard Rust
  benchmark regression trio there, and pulls the merged/per-matrix comparison
  reports back into `tmp/standard-benchmark/`. The native command also
  refreshes `tmp/standard-benchmark/index.json` plus `README.md` and prunes old
  run folders by retention count.
- `./scripts/build-standard-benchmark-index.mjs`
  Rebuild the local standard benchmark run index under `tmp/standard-benchmark/`
  and prune older run directories outside the retained window. This emits
  `index.json`, `README.md`, and `index.html`.
- `./scripts/build-nightly-artifact-overview.mjs`
  Rebuild the top-level `tmp/` nightly artifact overview across the direct-mesh,
  workflow-catalog, standard-benchmark, workflow-mesh, and exploratory
  benchmark-profile lanes. This emits `tmp/README.md`,
  `tmp/nightly-overview.json`, and `tmp/nightly-overview.html`.
- `./scripts/build-regression-lane-catalog.mjs --tmp-root tmp`
  Rebuild the normalized cross-lane regression catalog for the latest retained
  direct-mesh, workflow-catalog, workflow-mesh, and advisory benchmark-profile
  outputs. This emits
  `tmp/regression-lane-catalog.json`, `tmp/regression-lane-catalog.md`, and
  `tmp/regression-lane-catalog.html`, including a shared `gate` decision layer
  with per-lane reasons plus the catalog-level `overall_gate_status`. Advisory
  evidence lanes are visible but excluded from the enforced overall gate. The
  benchmark-profile lane reader lives in
  `build-regression-lane-catalog-profile.mjs` to keep the catalog builder below
  the source organization line limit.
- `./scripts/build-regression-gate-report.mjs --tmp-root tmp`
  Collapse the shared regression lane catalog into a CI/installer-friendly gate
  output. This emits `tmp/regression-gate-report.json` plus
  `tmp/regression-gate-report.md`, prints the overall gate status, exits `2`
  for `fail`, and can exit non-zero for `warn` via `--fail-on-warn`.
- `make benchmark-standard-nightly`
  Makefile entry for the remote standard benchmark regression flow against the
  checked-in standard baselines.
- `.github/workflows/standard-benchmark-nightly.yml`
  Self-hosted GitHub Actions entry for the remote standard benchmark
  regression flow and artifact upload path.
- `.github/workflows/workflow-mesh-nightly.yml`
  Self-hosted GitHub Actions entry for the remote workflow mesh regression
  flow, unified gate refresh, and artifact upload path.
- `.github/workflows/workflow-catalog-nightly.yml`
  Self-hosted GitHub Actions entry for the remote workflow catalog regression
  flow and artifact upload path.
- `.github/workflows/direct-mesh-docker-nightly.yml`
  Self-hosted GitHub Actions entry for the remote direct-mesh Docker
  regression flow, unified gate refresh, and artifact upload path.
- `cd apps/web && mix test test/kyuubiki_web/api/workflow_large_graph_api_test.exs && ELIXIR_PA="$(find "$PWD/_build/test/lib" -maxdepth 2 -type d -name ebin -print | tr '\n' ' ')" elixir -pa $ELIXIR_PA ../../scripts/workflow-large-graph-benchmark.exs 96 256 512 --output ../../tmp/workflow-large-graph-benchmark.json`
  Lower-level host script path for environments that allow plain Elixir TCP
  sockets outside `mix test`.
- `cd apps/web && mix test test/kyuubiki_web/api/workflow_catalog_thermal_job_api_test.exs test/kyuubiki_web/api/workflow_catalog_guard_job_api_test.exs && ELIXIR_PA="$(find "$PWD/_build/test/lib" -maxdepth 2 -type d -name ebin -print | tr '\n' ' ')" elixir -pa $ELIXIR_PA ../../scripts/workflow-catalog-benchmark.exs --repeat 5 --output ../../tmp/workflow-catalog-benchmark.json`
  Lower-level host script path for replaying the catalog-backed composite
  workflow benchmark without going through a dedicated `mix test` report case.

Examples now include:

- `hot-local`
- `hot-cloud`
- `hot-distributed`
- `hot-web`
- `hot-agent`
- `hot-hub-gui`
- `hot-installer-gui`
- `hot-workbench-gui`
- `build-frontend`
- `build-orchestrator`
- `build-agent`
- `build-hub-gui`
- `build-installer-gui`
- `build-workbench-gui`
- `package-runtime`
- `package-desktop`
- `desktop-upload-remote`
- `desktop-status`
- `desktop-stage`
- `desktop-build-host`
- `desktop-release`
- `desktop-verify`
- `desktop-linux-remote`
  Sync and run the Linux desktop packaging lane on `kyuubiki-lab`; use
  `desktop-linux-remote preflight` before the full build to check Node and
  Linux Tauri system dependencies.
- `desktop-linux-remote install-deps`
  Installer-aligned privileged dependency lane for the lab host. It uses
  `sudo -n`, so it fails cleanly instead of prompting for or storing a
  password.
- `sync-desktop-shared`
- `test-hub-gui`
- `test-installer-gui`
- `test-workbench-gui`

Keep these scripts thin. Product logic should live in the application/runtime
code, not in shell branching.

Hot-reload note:

- Next.js and Tauri already provide their own dev/HMR loops.
- `./scripts/kyuubiki hot-*` adds the missing restart-on-change layer for the
  non-Phoenix Elixir control plane and Rust solver agents so the whole stack
  can iterate under one operator command.
