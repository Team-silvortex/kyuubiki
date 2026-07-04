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
- `operator-task-ir.schema.json` is for dual-mode operator task descriptions
  authored by Elixir, Rust-native SDKs, or external SDKs. It now pins runtime
  hints, package-fetch semantics, and SHA-256 integrity field shape. Digest
  rules are in [operator-task-ir-digest.md](../docs/operator-task-ir-digest.md)
- `operator-task-batch.schema.json` is for `quality_execution_batch` payloads
  that group language-neutral Operator TaskIR envelopes for control-plane,
  SDK, orchestra, or agent execution.
- `operator-task-batch-preparation.schema.json` is for non-executing
  `prepare-batch` responses that validate batch manifests and expose per-task
  dispatch summaries before agent placement or execution.
- `operator-task-batch-checkpoint.schema.json` is for resumable batch-run
  manifests that bind a batch digest to preparation/execution summaries and a
  visible resume policy.
- `operator-task-batch-resume-plan.schema.json` is for agent/control-plane
  recovery plans derived from verified checkpoints, including target and
  blocked case lists for the next action.
- `operator-execution-program.schema.json` is for the language-neutral program
  contract carried inside operator task IR and consumed by agent engines,
  including solver RPC vs generic operator-task ABI consistency rules
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
- `examples.operator-task-ir.json` is the language-neutral golden TaskIR sample
  shared by schema readers, SDK smoke tests, and agent engine bring-up
- `examples.operator-task-batch.json` is the matching batch wrapper sample for
  `POST /api/v1/operator-tasks/execute-batch` and SDK batch execution examples
- `examples.operator-task-batch-preparation.json` is the matching
  `POST /api/v1/operator-tasks/prepare-batch` response sample for SDK and
  agent preflight tooling
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
