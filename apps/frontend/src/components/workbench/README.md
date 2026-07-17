# Workbench Components

This directory contains the browser workbench surface.

Top-level files keep the shell and cross-cutting surfaces:

- `workbench.tsx`
  Top-level state orchestration for the frontend workbench.
- `workbench-viewport.tsx`
  2D/3D visualization and interaction surface.
- `workbench-viewport-panel.tsx`
  Viewport panel shell for head actions, dock layout, result bar, and canvas stage.
- `workbench-inspector.tsx`
  Right-side diagnostics, properties, history, and reporting surface.
- `workbench-console.tsx`
  Bottom report/messages/results surface.
- `workbench-object-tree.tsx`
  Modeling-side object tree and selection list.
- `workbench-script-panel.tsx`
  Pwdt surface powered by WASM Python / Pyodide. This panel is for browser-side
  automation, macro recording, DSL compilation, and replay.
- `workbench-headless-workflow-panel.tsx`
  Separate headless SDK workflow builder for service-side and solver-side
  automation that bypasses the frontend UI.

Subdirectories group extracted workbench surfaces by domain:

- `study/`
  Study setup and run controls extracted from the main workbench shell. Keep
  these components focused on study configuration and execution interaction,
  not on shared domain transforms.
- `model/`
  Modeling-side shells and cards such as tools, materials, parametric generators,
  and 3D tree surfaces. Keep pure editing commands and stable view-model
  mappers in `src/lib/workbench` when they no longer need to live inside JSX.
- `library/`
  Sample/project/model/job library surfaces. Larger lists here should prefer
  virtualized or deferred rendering patterns.
- `system/`
  Runtime/config/data administration surfaces, including the system section shell
  and runtime panel composition. Reusable runtime metrics and observer cards
  should prefer this directory over growing `workbench.tsx`.

These files are intentionally grouped because they evolve together as one UI
domain even when they are rendered separately.

Implementation rules for this directory:

- `workbench.tsx` coordinates the shell and shared state, but should not absorb
  every domain transform or render helper forever
- heavy surfaces should split by visible responsibility
- viewport-specific interaction logic should stay close to the viewport surface
- domain transforms should prefer `src/lib` when they are reusable outside one
  render tree
- stable system subsections should grow as their own panel shells here instead
  of rebuilding the same card composition in the main workbench file

See:

- [docs/frontend-implementation.md](../../../../../docs/frontend-implementation.md)
