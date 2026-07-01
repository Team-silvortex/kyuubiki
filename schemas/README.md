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
- `operator-task-ir.schema.json` is for dual-mode operator task descriptions
  authored by Elixir, Rust-native SDKs, or external SDKs
- `operator-execution-program.schema.json` is for the language-neutral program
  contract carried inside operator task IR and consumed by agent engines
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

They are intentionally lightweight and JSON-first. They now serve four
consumers:

- frontend workbench
- Hub desktop shell
- orchestrator API
- Rust runtime/engine
- installer and deployment tooling
