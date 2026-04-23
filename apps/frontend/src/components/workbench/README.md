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
  WASM Python automation surface that drives registered frontend actions through
  a Pyodide bridge.

Subdirectories group extracted workbench surfaces by domain:

- `study/`
  Study setup and run controls extracted from the main workbench shell.
- `model/`
  Modeling-side shells and cards such as tools, materials, parametric generators,
  and 3D tree surfaces.
- `library/`
  Sample/project/model/job library surfaces.
- `system/`
  Runtime/config/data administration surfaces, including the system section shell
  and runtime panel composition.

These files are intentionally grouped because they evolve together as one UI
domain even when they are rendered separately.

Implementation rules for this directory:

- `workbench.tsx` coordinates the shell and shared state, but should not absorb
  every domain transform or render helper forever
- heavy surfaces should split by visible responsibility
- viewport-specific interaction logic should stay close to the viewport surface
- domain transforms should prefer `src/lib` when they are reusable outside one
  render tree

See:

- [docs/frontend-implementation.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-implementation.md)
