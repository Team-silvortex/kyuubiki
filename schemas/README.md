# Shared Schemas

These schemas capture the first cross-process contracts described in the root
README.

- `job.schema.json` is for durable job state
- `progress-event.schema.json` is for streamed runtime updates
- `model.schema.json` is for versioned browser model import/export payloads
- `material-library.schema.json` is for reusable material library import/export payloads
- `material-card.schema.json` is for first-class material cards with
  provenance, confidence, unit-system, parameter, and applicability metadata
- `examples.material-card.json` is the golden material-card fixture used by
  `make check-material-card-contract` to keep material provenance, units,
  confidence, and parameter shape aligned with workflow preflight
- `project.schema.json` is for portable `.kyuubiki.json` project bundles and
  standardized `.kyuubiki` archive manifests, including asset catalogs and meta
  sidecars plus guid reference graphs
- `agent-manifest.schema.json` is for local/distributed solver node manifests
- `language-pack.schema.json` is for Workbench-local and future remotely
  downloadable UI language packs
- `workflow-graph.schema.json` is for headless-first multi-operator workflow
  definitions, including node/edge wiring, typed ports, and portable workflow
  entry/output layout
- `workflow-dataset.schema.json` is for ONNX-like cross-operator data
  contracts, including named values, shape semantics, encoding, and stable
  schema references shared across workflow nodes
- `schemas/gui-runtime-capability-manifest.schema.json` is for GUI-to-runtime
  capability manifests that keep Hub, Workbench, Installer, browser WebView,
  and mobile WebView surfaces decoupled from orchestra, agent, mesh, direct
  runtime, and offline-bundle implementations
- `material-envelope-catalog-request.schema.json` is for cross-SDK material
  envelope catalog job requests that keep the built-in workflow graph owned by
  the central workflow catalog
- `material-candidate-materialization-plan.schema.json` is for reviewed
  material candidate specs that have been materialized by an agent or lab
  wrapper and are ready for solver rerun by any headless runner
- `material-study-execution-plan.schema.json` is for non-executing material
  study plans emitted before solver dispatch, including action order, solve
  step counts, candidate IDs, material-card contract metadata, and concrete
  workflow steps shared by CLI, SDK, and remote scheduling layers
- `material-exploration-chain.schema.json` is for repeated material
  exploration runs, including convergence assessment, optimization trace,
  repair planning, compact summaries, and retained per-round exploration
  artifacts
- `material-research-bundle.schema.json` is for the first retained automated
  material research artifact, tying an initial exploration, next-round
  execution plan, rerun, chained rounds, artifact checksums, and reproducible
  commands into one screening-level review bundle. Its summary mirrors the
  material-card references, embedded next-round decision, next iteration,
  runnable step count, and chain stop reason so agents can read the top-level
  state without losing artifact consistency. Its `research_evidence` block is a
  compact cross-check index for ranked candidates, optimization metrics,
  violated quality gates, focus candidates, plan step count, chain trace count,
  and final chain winner. Its `validation_evidence` block records screening
  baseline refs, confidence counts, sensitivity proxy metrics, acceptance
  criteria, uncertainty limits, validation-readiness decision, blocking reasons,
  and external validation requirements.
- `material-research-bundle-index.schema.json` is for the lightweight retained
  bundle index used by CI, release notes, and agents. It lists retained material
  studies, decision counts, winner drift, compact metric/gate evidence, focus
  candidates, chain trace counts, screening validation posture, baseline counts,
  acceptance-criteria counts, candidate confidence counts, and validation
  readiness summaries without embedding full solver payloads.
- `examples.material-research-bundle-index.json` is the compact fixture used by
  `make check-material-research-bundle-index-contract`.
- `operator-task-ir.schema.json` is for dual-mode operator task descriptions
  authored by Elixir, Rust-native SDKs, or external SDKs. It now pins runtime
  hints, package-fetch semantics, and SHA-256 integrity field shape. Digest
  rules are in [operator-task-ir-digest.md](../docs/operator-task-ir-digest.md)
- `operator-task-batch.schema.json` is for `quality_execution_batch` payloads
  that group language-neutral Operator TaskIR envelopes for control-plane,
  SDK, orchestra, or agent execution.
- `operator-task-batch-preparation.schema.json` is for non-executing
  `prepare-batch` responses that validate batch manifests and expose per-task
  dispatch summaries plus top-level `error_codes` / `error_code_counts` before
  agent placement or execution.
- `operator-task-batch-execution.schema.json` is for `execute-batch` responses
  that bind batch run metadata, per-case execution results, failed case IDs, and
  top-level `error_codes` / `error_code_counts` for large-run diagnostics.
- `operator-task-batch-checkpoint.schema.json` is for resumable batch-run
  manifests that bind a batch digest to preparation/execution summaries and a
  visible resume policy.
- `operator-task-batch-resume-plan.schema.json` is for agent/control-plane
  recovery plans derived from verified checkpoints, including target and
  blocked case lists for the next action.
- `operator-execution-program.schema.json` is for the language-neutral program
  contract carried inside operator task IR and consumed by agent engines,
  including solver RPC vs generic operator-task ABI consistency rules
- `operator-reliability-manifest.schema.json` is for the machine-readable
  reliability evidence index that lists release metadata, the release minimum
  coverage level gate, and per-domain reliability shards
- `operator-reliability-shard.schema.json` is for each per-domain reliability
  shard that maps physics-coverage solve operators to benchmark templates,
  tests, current trust level, explicit limitations, and the extra evidence
  required before any operator may claim `qualification`
- `operator-qualification-roadmap.schema.json` is for the release-owned
  qualification candidate queue that lists priority groups, evidence gaps,
  required artifacts, target trust level, evidence phase, primary blocker,
  preferred validation lane, release-gate impact, and graduation gates before
  selected review operators can move toward stronger trust
- `operator-qualification-evidence-kits.schema.json` is for the planning-grade
  artifact kits attached to qualification roadmap candidates before any of
  those artifacts are promoted into manifest-level `evidence.qualification`;
  generated bundle artifacts can include both capture and check commands
- `operator-qualification-readiness.schema.json` is for the generated
  qualification readiness report that turns roadmap candidates and evidence
  kits into a sorted next-action queue; it also carries validation profile
  mapping counts so component profiles and release-candidate profiles do not
  get conflated in coverage/status reporting. Its summary also exposes release
  review decisions and approved promotion-summary matching counts so the
  qualification gate can be audited without opening every evidence bundle.
- `operator-qualification-release-records.schema.json` is for release-bound
  qualification evidence records that bind snapshot metadata, candidate IDs,
  capture commands, check commands, retained evidence bundle paths, and the
  structured review status/gate that blocks or permits later promotion
- `operator-qualification-release-evidence.schema.json` is for retained
  qualification evidence bundles. It keeps command results, provenance,
  release-retention flags, and the promotion summary that binds retained
  evidence paths to review decisions, release records, and promoted operator
  IDs.
- `operator-qualification-review-decision.schema.json` is for reviewer-authored
  promotion decisions. It binds candidate ID, release version, evidence path,
  review gate, reviewer identity, decision, rationale, and requested changes
  before release records are allowed to move out of pending sign-off.
- `operator-validation-profiles.schema.json` is for the input profile contract
  consumed by `make check-operator-validation`, including grouped operators,
  evidence paths, validation methods, formal invariants, and controlled command
  kinds; each profile declares whether it is a release candidate profile or a
  component profile feeding a broader qualification candidate
- `operator-package-dynamic-smoke.schema.json` is for the retained
  end-to-end external operator package smoke report, including template tests,
  strict package preflight, template `cdylib` build, and engine dynamic host
  loading stage evidence with per-stage descriptions, working directories, and
  command vectors
- `operator-validation-report.schema.json` is for the machine-readable output
  of `make check-operator-validation` and `make verify-operator-validation`,
  including profile rollups, command kinds, skipped-command placeholders, and
  executed command tail diagnostics
- `workload-catalog.schema.json` is for Hub-facing workload libraries and
  future central-server downloadable project catalogs, including optional
  `analysis_domains` and `thermal_intents` hints that let Hub and Workbench
  classify workloads before opening them
- `central-store-contract-check.schema.json` is for the architecture-owned
  central store contract checker config. It keeps catalog, auth, publish,
  provenance, database, frontend client, docs, and readiness guard inputs
  project-relative and machine-checkable.
- `central-publish-pipeline.schema.json` is for the center-store write-side
  workflow contract that orders publisher identity, artifact envelope,
  signature attestation, review queue, catalog indexing, recall/yank, and
  installer verification before real upload endpoints are enabled.
- `component-integrity-report.schema.json` is for the generated component
  integrity report that turns `deploy/installation-integrity-contract.json`
  into machine-readable component counts, required-layout coverage, central
  service component summaries, and protocol issues.
- `contracts-runtime-api-surface.schema.json` is for the shared runtime API
  family map, including source files, client surfaces, internal service-surface
  bindings, verification commands, and repository-relative path constraints.
- `module-extension-standard.schema.json` is for the architecture extension
  onboarding flow, covering new modules, function paradigms, service surfaces,
  evidence lanes, and contract families before they enter release gates.
- `deploy/installation-integrity-contract.json` is the shared installer and
  desktop-facing installation contract source that defines standard layout,
  protected paths, cleanup allowlists, and visible repair rules
- `deploy/workload-catalog.example.json` is a concrete sample payload for local
  testing, Hub mockups, and future center-server rollout
- `examples.workflow-graph.json` is a minimal reference workflow for
  `heat -> thermo_mechanical` graph wiring
- `examples.workflow-dataset.json` is the matching reference dataset contract
  for that workflow's cross-operator payloads
- `schemas/examples.gui-runtime-capability-manifest.json` is the matching
  reference manifest for `kyuubiki.gui-runtime-capability-manifest/v1`, showing
  orchestrated, direct-mesh, and offline read-only GUI bindings without making
  the GUI own the runtime
- `config/gui-runtime-capabilities/*.json` contains the product-owned Hub,
  Workbench, Installer, and mobile WebView manifests validated against that
  GUI-to-runtime contract
- `examples.material-envelope-catalog-request.json` is the shared SDK fixture
  for submitting the built-in material envelope ranking workflow through the
  workflow catalog without embedding its graph inline
- `examples.material-candidate-materialization-plan.json` is the shared
  materialization fixture for passing reviewed composite-material candidates
  from an agent or custom wrapper into a solver rerun stage
- `examples.material-study-execution-plan.json` is the shared material study
  execution-plan fixture for validating `--plan-study` output, material-card
  contract metadata, and dispatch shape before solver dispatch or remote agent
  scheduling
- `examples.material-exploration-chain.json` is the shared chain fixture for
  validating `--chain-next` output, convergence assessment, optimization trace,
  and summary/run count alignment
- `examples.material-research-bundle.json` is the shared retained research
  bundle fixture for validating artifact checksums, reproduction command
  arrays, and the top-level screening research summary, including consistency
  between summary fields and embedded execution/chain artifacts
- `examples.operator-task-ir.json` is the language-neutral golden TaskIR sample
  shared by schema readers, SDK smoke tests, and agent engine bring-up
- `examples.operator-task-ir-float.json` is the fractional-number TaskIR sample
  that keeps canonical JSON digest behavior aligned across JS and Rust checks
- `examples.operator-task-ir-elixir.json` is the Elixir control-plane authored
  TaskIR sample proving hot authoring still lowers into language-neutral
  execution, package-fetch, mirror, and digest contracts
- `examples.operator-task-batch.json` is the matching batch wrapper sample for
  `POST /api/v1/operator-tasks/execute-batch` and SDK batch execution examples
- `examples.operator-task-batch-preparation.json` is the matching
  `POST /api/v1/operator-tasks/prepare-batch` response sample for SDK and
  agent preflight tooling
- `examples.operator-task-batch-execution.json` is the matching
  `POST /api/v1/operator-tasks/execute-batch` response sample for SDK and agent
  execution tooling
- `examples.operator-task-batch-checkpoint.json` is the matching resumable
  checkpoint sample for preserving batch-run state between distributed attempts
- `examples.operator-task-batch-resume-plan.json` is the matching recovery
  plan sample for turning a checkpoint resume policy into explicit next work
- `examples.operator-task-batch-blocked-checkpoint.json` and
  `examples.operator-task-batch-blocked-resume-plan.json` show the package or
  runtime readiness-blocked recovery path where the next action is
  `resolve_blocked_cases`, not ordinary failed-case retry
- `examples.operator-package-dynamic-smoke.json` is the retained report fixture
  for validating external operator package dynamic-smoke stage order,
  diagnostic working directories and commands, package/operator summary, and
  evidence paths
- `examples.operator-validation-report.json` is the retained fixture for the
  operator validation report shape, including the `boundary_regression` command
  kind and the non-executed `not_run` result form
- `examples.operator-qualification-review-decision.json` is the retained
  fixture for a reviewer decision that requests changes against a release
  qualification evidence bundle
- `examples.operator-qualification-release-evidence.json` is the retained
  fixture for the release evidence bundle contract, including the promotion
  summary used by line-field qualification promotion checks

They are intentionally lightweight and JSON-first. They now serve four
consumers:

- frontend workbench
- Hub desktop shell
- orchestrator API
- Rust runtime/engine
- installer and deployment tooling
