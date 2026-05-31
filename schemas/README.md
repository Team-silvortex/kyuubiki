# Shared Schemas

These schemas capture the first cross-process contracts described in the root
README.

- `job.schema.json` is for durable job state
- `progress-event.schema.json` is for streamed runtime updates
- `model.schema.json` is for versioned browser model import/export payloads
- `material-library.schema.json` is for reusable material library import/export payloads
- `project.schema.json` is for portable `.kyuubiki.json` project bundles and
  standardized `.kyuubiki` archive manifests, including asset catalogs and meta
  sidecars plus guid reference graphs
- `agent-manifest.schema.json` is for local/distributed solver node manifests
- `language-pack.schema.json` is for Workbench-local and future remotely
  downloadable UI language packs
- `workflow-graph.schema.json` is for headless-first multi-operator workflow
  definitions, including node/edge wiring, typed ports, and portable workflow
  entry/output layout
- `workload-catalog.schema.json` is for Hub-facing workload libraries and
  future central-server downloadable project catalogs, including optional
  `analysis_domains` and `thermal_intents` hints that let Hub and Workbench
  classify workloads before opening them
- `deploy/workload-catalog.example.json` is a concrete sample payload for local
  testing, Hub mockups, and future center-server rollout
- `examples.workflow-graph.json` is a minimal reference workflow for
  `heat -> thermo_mechanical` graph wiring

They are intentionally lightweight and JSON-first. They now serve four
consumers:

- frontend workbench
- Hub desktop shell
- orchestrator API
- Rust runtime/engine
- installer and deployment tooling
