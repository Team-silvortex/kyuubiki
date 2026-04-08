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
- `public/models/`
  Sample models bundled with the frontend.

This app should stay API-driven. It should consume control-plane, solver-RPC,
and schema contracts rather than backend implementation details.
