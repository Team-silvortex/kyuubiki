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

These files are intentionally grouped because they evolve together as one UI
domain even when they are rendered separately.
