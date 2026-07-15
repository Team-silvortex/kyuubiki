# Scripts

This directory contains host-native operational entry points.

- `kyuubiki`
  Thin compatibility shim for the native Rust `kyuubiki-script-runner`
  binary. New operational command logic should land in
  `workers/rust/crates/script-runner`, not in shell.
- `kyuubiki-script-runner frontend-file-lines` and
  `kyuubiki-script-runner frontend-storage-security`
  Native Rust replacements for the former frontend Node check scripts. The
  npm `check:file-lines` and `check:storage-security` entries still exist as
  developer-friendly package scripts, but their implementation now lives in
  `workers/rust/crates/script-runner/src/frontend_checks.rs`.
- `kyuubiki-lab`
  Thin operational wrapper for the shared Ubuntu lab machine that now hosts
  the standard download/deploy server plus the shared solver-agent test node.
- `kyuubiki-script-runner create-release-snapshot --self-test`
  Native release-snapshot gate self-test for required frontend/repo checks and
  source-version issue shape. The retained `.mjs` script still owns scaffold
  writes for now: it creates a release snapshot manifest and updates the release
  index. When a snapshot is marked `current`, it also advances the shared
  shipping-version contracts. Current snapshots require package, Tauri, brand,
  and Rust workspace source versions to match the requested version before the
  snapshot is written, so release metadata cannot capture stale product-surface
  versions.
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
- `kyuubiki-script-runner audit-version-line`
  Audit repository-wide version contracts and inventory visible version
  references before advancing a shipping line such as `tamamono 1.7.0`. The
  exact-contract lane includes release metadata, package metadata, generated
  docs mirrors, update catalogs, shipped language-pack catalog versions, and
  hand-maintained Markdown facts such as `current-line.md` and
  `version-line.md`. The retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner rust-line-audit`
  Native Rust source line-count audit, currently `800` lines per file by
  default, so crate and test modules stay split before they become hard to
  review. The command keeps the old `--root`, `--max`, and `--json` options.
- `kyuubiki-script-runner audit-local-paths`
  Native tracked-file scan that rejects local machine absolute paths such as
  user-home paths, macOS temporary-folder paths, mounted-volume paths, and
  host package-manager prefixes in source, docs, schemas, and scripts.
- `kyuubiki-script-runner check-module-extension-standard`
  Native contract check for the standard flow used when adding modules,
  function paradigms, service surfaces, evidence lanes, and contract families.
  It preserves the `--self-test` lane used by `make check-module-extension-standard`.
- `kyuubiki-script-runner validate-material-score-contract`
  Native material-scoring contract check. It keeps the manifest, Markdown,
  Elixir runtime, Rust TaskIR runtime, and tests aligned around
  `transform.score_material_candidates`.
- `kyuubiki-script-runner check-doc-book`
  Native docs-book and Hub mirror check. It replaces the Make-level Node
  invocation for centralized book version alignment, required chapter markers,
  local-link validation, and legacy wording rejection. The old
  `check-doc-book.mjs` remains as a compatibility script for direct callers.
- `kyuubiki-script-runner sync-doc-book-version`
  Native docs-book version synchronizer used by `make sync-doc-book-version`.
  It keeps the `--version` and `--line` options from the legacy Node script
  while making the standard Make path independent of Node.
- `kyuubiki-script-runner check-toolchain-contract`
  Native toolchain drift check for `config/toolchains.json`, Rust toolchain
  pins, Docker bases, Elixir constraints, embedded runtime references, remote
  defaults, and package Node engines. The legacy
  `check-toolchain-contract.mjs` remains available for direct compatibility.
- `kyuubiki-script-runner check-install-update-disk-hygiene`
  Native install/update disk-use contract check. It keeps release upload,
  remote artifact authority, local purge allowlists, rollback visibility, and
  installation-integrity references aligned while preserving the old
  `--self-test` lane.
- `kyuubiki-script-runner check-module-topology`
  Native architecture topology check for module layers, ownership paths,
  benchmark/security lanes, lane test plans, service surfaces, and acyclic
  module dependencies. It preserves the old `--self-test` lane while moving the
  Make-level path off Node.
- `kyuubiki-script-runner build-module-topology-report`
  Native module topology report generator. It writes the retained JSON,
  Markdown, and HTML topology report while preserving `--topology` and
  `--out-dir`.
- `kyuubiki-script-runner check-module-function-matrix`
  Native module x function-paradigm coverage matrix check. It validates module
  rows against the topology, required paradigm cells, status values, and writes
  the retained JSON/Markdown matrix report while preserving `--out` and
  `--self-test`.
- `kyuubiki-script-runner check-module-function-coverage-tensor`
  Native module x function-paradigm x evidence-depth tensor generator. It
  validates tensor lane mappings and contract evidence, derives gap severity,
  and writes the retained JSON/Markdown tensor report while preserving `--out`
  and `--self-test`.
- `kyuubiki-script-runner audit-project-organization`
  Enforce the repository-wide source organization guard. New source and docs
  files, including untracked files that are not ignored, stay under the shared
  line ceiling, while explicitly tracked historical debt files are allowed only
  up to their current debt limit. It also keeps installer `tests.rs` as a
  module index instead of a growing test bucket. Use `--self-test` to verify
  the audit helper rules themselves.
- `kyuubiki-script-runner audit-dependencies`
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
  Legacy compatibility entry for the docs-book check. Prefer
  `./scripts/kyuubiki check-doc-book` or `make check-doc-book` for new
  automation.
- `kyuubiki-script-runner check-elixir-self-host`
  Verify the Elixir/Mix/OTP runtime plus the orchestrator self-host
  environment contract before a machine is treated as installer-managed.
- `kyuubiki-script-runner check-make-modules`
  Native Rust check that verifies the root `Makefile` stays as a small
  include-based entrypoint, that every `make/*.mk` module is included, and
  that retired catch-all modules do not come back. The `make
  check-make-modules` target calls this runner command directly.
- `kyuubiki-script-runner validate-language-packs`
  Validate the shipped Workbench/Hub language support pack catalog and JSON
  envelopes for the current release index version in the `tamamono 1.x` line,
  including exact parity with
  `config/localization/mainstream-language-pack-locales.json`. Make now uses
  the native runner; the retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner check-ui-automation-contract`
  Verify the product-owned Workbench UI automation selector contract across the
  JSON contract, HTML documentation, TypeScript helper, and implementation
  files. Make now uses the native runner; the retained `.mjs` script is only a
  parity reference.
- `kyuubiki-script-runner check-gui-runtime-capability-contract`
  Verify GUI-to-runtime capability manifests, mobile WebView boundaries,
  frontend capability helpers, and Workbench backend-service indirection. Make
  now uses the native runner; the retained `.mjs` script is only a parity
  reference.
- `kyuubiki-script-runner check-contracts-runtime-api-surface`
  Verify the contracts-owned runtime API surface, including required contract
  families, central self-host service surface binding, repo-relative evidence
  paths, and schema-side service surface/repo path guards. Make now uses the
  native runner; the retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner check-central-store-contract`
  Verify the future central-server store contract across JSON schemas, Elixir
  API surface, frontend API client/types, tests, and docs. It also keeps the
  read-only central database status and provenance policy endpoints aligned
  with the storage table, publish-readiness, and artifact verification
  contracts before write-side publishing exists. The checked surface is driven
  by `config/architecture/central-store-contract.json` so new center-server
  routes can be added without growing the script itself. Make now uses the
  native runner; the retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner check-central-database-readiness`
  Verify central-server database readiness before local or server deployment
  smoke tests. It checks storage mode, required env, and DB policy surfaces
  without opening a network/database connection. Make now uses the native
  runner; the retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner build-central-readiness-report`
  Write a retained machine-readable central readiness report under `tmp/`.
  The report combines central DB readiness, API endpoint coverage, schema file
  presence, storage table-contract presence, and safe runbook commands without
  storing credentials. It also writes a compact Markdown summary for human
  review. Make now uses the native runner; the retained `.mjs` script is only a
  parity reference.
- `kyuubiki-script-runner check-central-readiness-report`
  Validate a retained central readiness report, including required endpoints,
  schema coverage, storage table-contract coverage, and absence of obvious
  inline credential material. Make now uses the native runner.
- `kyuubiki-script-runner check-verification-evidence-surface`
  Verify the verification-evidence runtime surface, including stable evidence
  commands, generated artifacts under `tmp/`, and the central readiness report
  generation/check pair. The retained `.mjs` script is kept only as a parity
  reference while Make uses the native runner.
- `run-central-database-smoke.mjs`
  Run the central-store database smoke wrapper. It always runs readiness first;
  by default it is a dry-run, and only executes Postgres-backed Elixir tests
  when `RUN_DB_SMOKE=1` or `--run` is supplied.
- `run-remote-central-database-smoke.mjs`
  Sync the current source tree to an existing SSH host such as `kyuubiki-lab`
  and run the same central DB readiness/smoke pair on that machine. It uses a
  relative scratch directory, excludes generated build outputs, and never
  stores SSH credentials or `DATABASE_URL` in the repository.
- `kyuubiki-script-runner check-ui-automation-contract`
  Verify the product-owned Workbench automation selector contract. It compares
  `docs/ui-automation-contract.json`, the frontend TS selector constants, and
  the component implementation anchors used by wasm-python and UI smoke tests.
  Use `--self-test` when changing selector coverage. The retained `.mjs` script
  is only a parity reference.
- `kyuubiki-script-runner check-operator-task-ir-contract`
  Verify the language-neutral TaskIR schema extension and shipped TaskIR
  examples. It checks that mirrored fields such as operator kind, package ref,
  and package version stay consistent across descriptor, execution program, and
  runtime hints, and recomputes canonical `descriptor_digest` and `task_digest`
  values, including a fractional-number fixture, before agent or SDK tests need
  to run. Make now uses the native runner; the retained `.mjs` script is only a
  parity reference.
- `kyuubiki-script-runner check-workflow-dataset-contract`
  Verify the ONNX-like workflow dataset schema, standalone example, workflow
  graph embedded dataset contract, graph port/edge dataset references, and the
  runtime rule documentation. It mirrors the Rust engine's dataset-contract
  validation so broken cross-operator value metadata is caught before workflow
  execution. Make now uses the native runner; the retained `.mjs` script is
  only a parity reference.
- `kyuubiki-script-runner check-materialization-plan-contract`
  Verify the shared material candidate materialization plan schema, fixture,
  and SDK documentation links. It keeps reviewed agent/lab materialization
  output aligned with the solver-rerun runner contract before SDK parity work
  consumes the same artifact. Make now uses the native runner; the retained
  `.mjs` script is only a parity reference.
- `kyuubiki-script-runner check-material-study-execution-plan-contract`
  Verify the shared non-executing material study execution plan schema,
  fixture, and SDK documentation links. It keeps `--plan-study` output aligned
  with headless SDK and remote scheduler expectations before solver dispatch.
  Make now uses the native runner; the retained `.mjs` script is only a parity
  reference.
- `build-material-research-bundle.mjs`, `kyuubiki-script-runner check-material-research-bundle`,
  `build-material-research-bundle-index.mjs`, and
  `check-material-research-bundle-contract.mjs`
  Build and verify the first retained material research bundle. The bundle
  captures initial exploration, next-round execution planning, a rerun,
  chained rounds, artifact checksums, and repo-relative reproduction commands
  under `kyuubiki.material-research-bundle/v1`. The checker verifies that the
  top-level summary matches the embedded next-round decision, next iteration,
  runnable step count, and chain stop reason. Override `STUDY=` through Make to
  build the heat-spreader or composite thermo-electric panel retained profile.
  The bundle index builder writes a compact multi-study overview under
  `tmp/material-research-bundles/` for CI, agents, and release notes, including
  next iteration and runnable next-step count for scheduling. The lightweight
  contract check keeps
  `schemas/material-research-bundle.schema.json`,
  `schemas/examples.material-research-bundle.json`, and documentation links in
  sync without running the solver. Make now uses the native runner for runtime
  and contract checks; retained `.mjs` checkers are only parity references.
- `operator-reliability-*.mjs` and `check-operator-reliability*.mjs`
  Operator reliability gate family. `operator-reliability-contracts.mjs`
  centralizes config/schema paths and schema versions,
  `operator-reliability-rules.mjs` owns pure trust-level and qualification
  rules, `kyuubiki-script-runner check-operator-reliability-rules` tests those
  rules with a native parity self-test,
  `kyuubiki-script-runner check-operator-reliability-schemas` runs the
  zero-dependency schema smoke, and `check-operator-reliability.mjs` performs the manifest,
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
- `kyuubiki-script-runner check-line-field-closed-form-baseline`
  Verify the first versioned qualification evidence artifact for the
  `line-field-closed-form` candidate. It checks that all four 1D closed-form
  operator baselines are present, finite, tolerance-bearing, and linked to the
  Rust accuracy-baseline tests. It also checks the candidate tolerance policy
  so tight closed-form tolerances cannot silently expand into broader claims.
  The retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner capture-line-field-qualification-provenance`
  Emit a repo-relative provenance JSON envelope for the same candidate. The
  output records revision state, toolchain versions, platform metadata, command
  contracts, and hashes of the evidence inputs without embedding local absolute
  paths. The retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner capture-line-field-qualification-release-evidence`
  Run the line-field evidence checker and Rust solver baseline, then retain
  sanitized command status, duration, output, and provenance in a repo-local
  JSON bundle. The retained `.mjs` script is only a parity reference.
- `kyuubiki-script-runner check-line-field-qualification-release-evidence`
  Validate a retained line-field release evidence bundle. It checks schema
  version, command success, provenance inputs, SHA-256 shape, release-retention
  flags, and absence of local absolute repository paths. The retained `.mjs`
  script is only a parity reference.
- `kyuubiki-script-runner build-operator-qualification-readiness`
  Build a repo-local JSON readiness report for all qualification roadmap
  candidates, including artifact state, graduation gates, and a sorted
  `next_actions` queue for the highest-priority evidence collection steps.
- `kyuubiki-script-runner check-operator-qualification-readiness`
  Validate the generated readiness report and its `next_actions` queue. Use
  `--self-test` when changing readiness sorting or action-kind requirements.
  The retained `.mjs` scripts are parity references.
- `kyuubiki-script-runner validate-commercial-readiness`
  Verify the `2.0` commercial-readiness manifest against its Markdown gate,
  including gate count, evidence links, and the shared exit statement.
- `kyuubiki-script-runner validate-minimal-industrial-closure`
  Verify the narrower `1.15.x -> 1.20.x` minimum industrial closure manifest
  against its Markdown gate, including gate count, evidence links, supported
  state values, and the shared exit statement.
- `sync-doc-book-version.mjs`
  Update the hand-maintained book entry pages to the current development version,
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
- Treat embedded `sh -lc`, `bash -lc`, or `ExecStart=/bin/sh` usage in runtime
  sources as a failing audit unless it is replaced by native argument vectors
  or an explicitly bounded host-tool boundary.
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
  match the current development version.
- `make check-operator-reliability`
  Check that every `physics-coverage` solve operator has a reliability manifest
  entry with benchmark, headless workflow, evidence, visible limitations, and a
  coverage level that satisfies the release minimum gate. The Make target runs
  the focused rule, schema smoke, and line-field closed-form baseline checks
  first.
- `make check-operator-reliability-rules`
  Run only the pure reliability rule self-test without loading benchmark
  catalogs, workflow payloads, manifest shards, or evidence files. Make now
  uses the native runner; the retained `.mjs` test is only a parity reference.
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
  validation. Make now uses the native runner; the retained `.mjs` script is
  only a parity reference.
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
  fields, optimization trace fields, or SDK chain documentation. Make now uses
  the native runner; the retained `.mjs` script is only a parity reference.
- `make check-material-research-bundle-contract`
  Run the zero-dependency retained material research bundle contract check and
  its self-test. Use this after changing
  `schemas/material-research-bundle.schema.json`,
  `schemas/examples.material-research-bundle.json`, retained summary fields,
  next-round execution-plan fields, or bundle documentation. Make now uses the
  native runner; the retained `.mjs` script is only a parity reference.
- `make material-research-bundle-index`
  Build the retained heat-spreader and composite thermo-electric panel bundles,
  validate them, and write `tmp/material-research-bundles/index.json` plus a
  human-readable `README.md` summary with next iteration and runnable next-step
  counts.
- `make build-operator-qualification-readiness`
  Write and validate a readiness report for the qualification roadmap. Override
  `OUT=tmp/name.json`; the report is a local planning artifact and should stay
  out of Git unless deliberately retained with a release. Make now uses the
  native runner for both build and validation.
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
  contract against `config/toolchains.json`. Use `./scripts/kyuubiki
  check-elixir-self-host --static-only --json` when preparing an installer
  image where Elixir is not yet installed.

Useful smoke wrappers:

- `./scripts/kyuubiki smoke`
  Current Elixir -> Rust integration smoke flow.
- `./scripts/kyuubiki sdk-smoke`
  Python / Elixir / Rust headless SDK smoke suite.
- `./scripts/kyuubiki agent-capability-smoke --host 192.0.2.12 --port 5001 --output tmp/agent-capability-smoke-5001.json`
  Probe a running solver agent, read its advertised RPC methods, and run the
  matching minimal Python SDK solver fixtures. This is the preferred quick
  check for installer-managed lab agents because it reports both tested and
  untested advertised methods without mutating the remote service.
- `AGENT_HOST=192.0.2.12 AGENT_PORT=5001 AGENT_SMOKE_PROFILE=lab-legacy-26 make test-agent-capability-smoke`
  Run the same check through Make with an explicit release gate. Raise
  `AGENT_SMOKE_PROFILE` to `current-40` for a local `1.20.x` agent with the
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
  rejected package reasons, package readiness issue counts, and a safety block
  confirming that dynamic libraries were not loaded.
- `make operator-package-preflight`
  Runs the same admission report against the checked-in Rust operator crate
  template under `workers/rust/templates/`.
- `make operator-package-preflight OUT=tmp/operator-package-preflight.json`
  Writes the same report to a repo-root-relative JSON file for CI artifacts or
  installer diagnostics.
- `make operator-package-preflight FAIL_ON_REJECTED=1`
  Turns rejected external operator packages into a non-zero quality gate.
- `cargo run -p kyuubiki-installer -- operator-package-preflight ./operator-packages --fail-on-readiness-warnings`
  Turns readiness warnings, such as `unverified` package status, into a
  non-zero release-readiness gate.
- `make operator-package-dynamic-smoke`
  Runs the repository template operator as an end-to-end external package:
  template tests, strict preflight, `cdylib` build, and engine dynamic host
  loading smoke. The default report is
  `tmp/operator-package-dynamic-smoke.json`; override with `OUT=tmp/name.json`.
- `make check-operator-package-dynamic-smoke IN=tmp/operator-package-dynamic-smoke.json`
  Validates the retained dynamic-smoke report schema, package/operator
  summary, stage order, stage descriptions, repo-local working directories,
  reproducible command vectors, stage success, and repo-local evidence paths.
  Make now uses the native runner; the retained `.mjs` script is only a parity
  reference.
- `make check-operator-package-dynamic-smoke-contract`
  Validates the shared dynamic-smoke schema and fixture without requiring a
  freshly generated `tmp/` report. Make now uses the native runner.
- `./scripts/kyuubiki desktop-upload-remote macos`
  Upload the current shipping-version desktop release outputs to the remote
  download server. Override the target with
  `KYUUBIKI_RELEASE_REMOTE_HOST=user@host`. Prefer SSH keys or an agent; the
  temporary `KYUUBIKI_RELEASE_REMOTE_PASSWORD=...` compatibility path requires
  `KYUUBIKI_RELEASE_REMOTE_ALLOW_PASSWORD=1` and uses `sshpass -e`; `PURGE_LOCAL=1` removes local `dist/` and platform-matched
  Tauri bundle outputs after a successful upload.
- `./scripts/run-direct-mesh-benchmark-container.sh --repeat 3`
  Compatibility shim for
  `./scripts/kyuubiki direct-mesh-benchmark-container --repeat 3`. It builds
  the dedicated Docker harness, runs the direct-mesh integration suite multiple
  times, and writes JSON plus Markdown summaries under
  `tmp/direct-mesh-benchmark-container/`. For LAN agent discovery, prefer
  `DOCKER_RUN_NETWORK=host`. The current checked-in baseline snapshot is
  `tests/integration/benchmarks/direct-mesh-docker-baseline.json`.
- `./scripts/kyuubiki compare-direct-mesh-benchmark --current tmp/direct-mesh-benchmark-container/latest/summary.json --baseline tests/integration/benchmarks/direct-mesh-docker-baseline.json --report-out tmp/direct-mesh-benchmark-container/latest/compare.md --json-out tmp/direct-mesh-benchmark-container/latest/compare.json`
  Compare a direct-mesh Docker benchmark summary against the checked-in
  baseline and emit both Markdown and machine-readable diff artifacts.
- `./scripts/run-direct-mesh-benchmark-regression.sh`
  Compatibility shim for
  `./scripts/kyuubiki direct-mesh-benchmark-regression`. It runs the remote
  direct-mesh Docker benchmark on `kyuubiki-lab`, copies the resulting summary
  back into the local workspace, and compares it against the checked-in
  baseline with regression thresholds. This native command expects a narrow
  passwordless `sudo` rule for the benchmark wrapper on the remote lab host.
- Benchmark and regression script details live in
  [BENCHMARKS.md](./BENCHMARKS.md) so this directory index stays below the
  project source organization limit.
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
  the source organization line limit. Project organization uses an `800` line
  default for source-like files and a `2000` line default for documentation
  files such as Markdown and HTML.
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

Examples include `hot-local`, `hot-cloud`, `hot-distributed`, `hot-web`,
`hot-agent`, desktop GUI hot loops, component builds, `package-runtime`,
`package-desktop`, desktop release/status/stage/verify commands, shared desktop
sync, and GUI smoke-test wrappers. `desktop-linux-remote` syncs and runs the
Linux desktop packaging lane on `kyuubiki-lab`; use
`desktop-linux-remote preflight` before the full build, and
`desktop-linux-remote install-deps` for the installer-aligned privileged
dependency lane. It uses `sudo -n`, so it fails cleanly instead of prompting for
or storing a password.

Keep these scripts thin. Product logic should live in the application/runtime
code, not in shell branching.

Hot-reload note:

- Next.js and Tauri already provide their own dev/HMR loops.
- `./scripts/kyuubiki hot-*` adds the missing restart-on-change layer for the
  non-Phoenix Elixir control plane and Rust solver agents so the whole stack
  can iterate under one operator command.
