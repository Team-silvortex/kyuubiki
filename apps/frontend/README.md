# Frontend Workbench

`apps/frontend` is the browser-facing workbench.

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

This app should stay API-driven. It should consume orchestrator and schema
contracts rather than backend implementation details.
