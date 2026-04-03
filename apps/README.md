# Apps

Top-level application surfaces live here.

- `frontend/`
  Next.js workbench UI. This is the browser-facing modeling, review, and 3D
  interaction layer.
- `installer-gui/`
  Tauri desktop installer and deployment control GUI.
- `web/`
  Elixir orchestrator API. This is the control plane for jobs, storage, result
  chunking, health, watchdog, and distributed agent coordination.

The `apps/` directory is intentionally product-facing. Shared compute/runtime
code lives outside this tree in `workers/` and `schemas/`.
