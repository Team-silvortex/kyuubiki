# Shared Schemas

These schemas capture the first cross-process contracts described in the root
README.

- `job.schema.json` is for durable job state
- `progress-event.schema.json` is for streamed runtime updates
- `model.schema.json` is for versioned browser model import/export payloads
- `material-library.schema.json` is for reusable material library import/export payloads
- `project.schema.json` is for portable `.kyuubiki.json` project bundles
- `agent-manifest.schema.json` is for local/distributed solver node manifests

They are intentionally lightweight and JSON-first. They now serve four
consumers:

- frontend workbench
- orchestrator API
- Rust runtime/engine
- installer and deployment tooling
