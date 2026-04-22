# Workbench Components

This directory contains the browser workbench surface:

- `workbench.tsx`
  Top-level state orchestration for the frontend workbench.
- `workbench-viewport.tsx`
  2D/3D visualization and interaction surface.
- `workbench-inspector.tsx`
  Right-side diagnostics, properties, history, and reporting surface.
- `workbench-console.tsx`
  Bottom report/messages/results surface.
- `workbench-object-tree.tsx`
  Modeling-side object tree and selection list.
- `workbench-script-panel.tsx`
  WASM Python automation surface that drives registered frontend actions through
  a Pyodide bridge.
- `workbench-study-sidebar.tsx`
  Study setup and run controls surface extracted from the main workbench shell.
- `workbench-model-sidebar.tsx`
  Model editing shell for tools/tree tabs extracted from the main workbench shell.
- `workbench-material-library-card.tsx`
  Material editing and import/apply card extracted from the model tools surface.
- `workbench-parametric-card.tsx`
  Parametric generator card extracted from the model tools surface.
- `workbench-library-sidebar.tsx`
  Sample/project/model/job library surface extracted from the main workbench shell.

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
