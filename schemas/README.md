# Shared Schemas

These schemas capture the first cross-process contracts described in the root
README.

- `job.schema.json` is for durable job state
- `progress-event.schema.json` is for streamed runtime updates
- `model.schema.json` is for versioned browser model import/export payloads
- `material-library.schema.json` is for reusable material library import/export payloads
- `material-card.schema.json` is for first-class material cards with
  provenance, confidence, unit-system, parameter, and applicability metadata
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
- `material-envelope-catalog-request.schema.json` is for cross-SDK material
  envelope catalog job requests that keep the built-in workflow graph owned by
  the central workflow catalog
- `material-candidate-materialization-plan.schema.json` is for reviewed
  material candidate specs that have been materialized by an agent or lab
  wrapper and are ready for solver rerun by any headless runner
- `material-study-execution-plan.schema.json` is for non-executing material
  study plans emitted before solver dispatch, including action order, solve
  step counts, candidate IDs, and concrete workflow steps shared by CLI, SDK,
  and remote scheduling layers
- `material-exploration-chain.schema.json` is for repeated material
  exploration runs, including convergence assessment, optimization trace,
  repair planning, compact summaries, and retained per-round exploration
  artifacts
- `material-research-bundle.schema.json` is for the first retained automated
  material research artifact, tying an initial exploration, next-round
  execution plan, rerun, chained rounds, artifact checksums, and reproducible
  commands into one screening-level review bundle
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
  required artifacts, and graduation gates before selected review operators can
  move toward `qualification`
- `operator-qualification-evidence-kits.schema.json` is for the planning-grade
  artifact kits attached to qualification roadmap candidates before any of
  those artifacts are promoted into manifest-level `evidence.qualification`
- `workload-catalog.schema.json` is for Hub-facing workload libraries and
  future central-server downloadable project catalogs, including optional
  `analysis_domains` and `thermal_intents` hints that let Hub and Workbench
  classify workloads before opening them
- `deploy/installation-integrity-contract.json` is the shared installer and
  desktop-facing installation contract source that defines standard layout,
  protected paths, cleanup allowlists, and visible repair rules
- `deploy/workload-catalog.example.json` is a concrete sample payload for local
  testing, Hub mockups, and future center-server rollout
- `examples.workflow-graph.json` is a minimal reference workflow for
  `heat -> thermo_mechanical` graph wiring
- `examples.workflow-dataset.json` is the matching reference dataset contract
  for that workflow's cross-operator payloads
- `examples.material-envelope-catalog-request.json` is the shared SDK fixture
  for submitting the built-in material envelope ranking workflow through the
  workflow catalog without embedding its graph inline
- `examples.material-candidate-materialization-plan.json` is the shared
  materialization fixture for passing reviewed composite-material candidates
  from an agent or custom wrapper into a solver rerun stage
- `examples.material-study-execution-plan.json` is the shared material study
  execution-plan fixture for validating `--plan-study` output before solver
  dispatch or remote agent scheduling
- `examples.material-exploration-chain.json` is the shared chain fixture for
  validating `--chain-next` output, convergence assessment, optimization trace,
  and summary/run count alignment
- `examples.material-research-bundle.json` is the shared retained research
  bundle fixture for validating artifact checksums, reproduction command
  arrays, and the top-level screening research summary
- `examples.operator-task-ir.json` is the language-neutral golden TaskIR sample
  shared by schema readers, SDK smoke tests, and agent engine bring-up
- `examples.operator-task-ir-float.json` is the fractional-number TaskIR sample
  that keeps canonical JSON digest behavior aligned across JS and Rust checks
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

They are intentionally lightweight and JSON-first. They now serve four
consumers:

- frontend workbench
- Hub desktop shell
- orchestrator API
- Rust runtime/engine
- installer and deployment tooling
