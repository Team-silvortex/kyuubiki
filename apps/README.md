# Apps

Top-level application surfaces live here.

## Quick Map

- `frontend/`
  Browser workbench and direct-mesh-capable GUI surface.
- `desktop-shared/`
  Shared frontend helper and asset sync source for the desktop Tauri app family.
- `web/`
  Phoenix/Plug control plane for jobs, persistence, results, and agent
  orchestration.
- `installer-gui/`
  Tauri installer/operator console for local, cloud, and distributed setups.
- `hub-gui/`
  Unified desktop launcher and runtime control shell for the whole Kyuubiki
  workstation.
- `workbench-gui/`
  Tauri desktop shell that wraps the browser workbench for native use.

## Ownership Boundary

- `frontend/`
  Next.js workbench UI. This is the browser-facing modeling, review, and 3D
  interaction layer.
- `desktop-shared/`
  Source-of-truth desktop frontend helper layer. It syncs shared Tauri-UI
  assets into `hub-gui`, `installer-gui`, and `workbench-gui`.
- `installer-gui/`
  Tauri desktop installer and deployment control GUI.
- `hub-gui/`
  Desktop orchestration shell that sits above installer and workbench surfaces
  as the everyday GUI entrypoint. It now serves as the operator-facing desktop
  entry shell for runtime control, workload launch, release readiness, and
  hot-reload watch flows.
- `workbench-gui/`
  Tauri desktop shell for the local engineering workbench. It embeds the
  browser workbench inside a native window and exposes local runtime controls.
- `web/`
  Elixir orchestrator API. This is the control plane for jobs, storage, result
  chunking, health, watchdog, and distributed agent coordination.

The `apps/` directory is intentionally product-facing. Shared compute/runtime
code lives outside this tree in `workers/` and `schemas/`.

The three desktop-facing Tauri apps are intended to evolve as one family:

- `hub-gui/` for desktop entry and orchestration
- `installer-gui/` for setup, deployment, and heavier operator flows
- `workbench-gui/` for focused modeling and analysis

See also:

- [docs/repository-structure.md](../docs/repository-structure.md)
- [docs/testing-and-ci.md](../docs/testing-and-ci.md)
- [docs/system-overview.md](../docs/system-overview.md)
