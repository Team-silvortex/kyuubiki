# Frontend Workbench

`apps/frontend` is the browser-facing workbench.

The workbench is now expected to support two runtime shapes over time:

- `orchestrated_gui`
  Uses the Phoenix control plane as the primary API and cluster coordinator.
- `direct_mesh_gui`
  Talks directly to headless Rust solver agents on a LAN peer mesh when a
  central orchestrator is not required.

Key subtrees:

- `src/app/`
  Next.js app entry points and global styling.
- `src/components/workbench/`
  Domain-specific workbench surfaces such as viewport, inspector, report, and
  object-tree panels.
- `src/components/ui/`
  Generic reusable UI primitives.
- `src/lib/`
  Browser-side API clients, import/export helpers, materials, and model logic.
- `src/lib/scripting/`
  Pyodide/WASM Python runtime helpers and the frontend automation action catalog.
- `public/models/`
  Sample models bundled with the frontend.

Automation support:

- The System panel now exposes a `Scripts` surface powered by WASM Python
  (Pyodide).
- The assistant surface now supports two execution modes:
  `local` for built-in rule/diagnostic guidance, and `llm` for OpenAI-compatible
  remote model planning.
- LLM-assisted execution is guarded by an explicit approval step before actions
  can run.
- Executed assistant plans are grouped into rollbackable frontend transactions.
- Frontend operations are registered behind a script action bridge so browser
  workflows can be automated without coupling to backend internals.
- The scripting bridge exposes live state polling helpers such as
  `wait_until`, `wait_for_job_done`, and `wait_for_message`.
- Scripted frontend actions are recorded into a lightweight action log inside
  the script panel so future assistants can replay or audit what they did.
- The first script run downloads the Pyodide runtime into the browser cache.

This app should stay API-driven. It should consume control-plane, solver-RPC,
and schema contracts rather than backend implementation details.
