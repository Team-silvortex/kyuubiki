# Frontend Library Modules

Browser-side support code is organized by domain:

- `api/`
  Orchestrator API client types and request helpers.
- `materials/`
  Material presets, parsing, and browser-side material library helpers.
- `models/`
  Model import/export, parametric generation, and sample-library helpers.
- `projects/`
  Portable project-bundle import/export helpers.
- `workbench/`
  Workbench-facing pure helpers for settings, serialization, export, and other
  non-JSX logic shared by heavy frontend surfaces.
  This also includes result-window sizing, offset, and chunk-cache helpers for
  large-result browsing.
  Material editing commands should also live here when they can stay pure.
  Snapshot/history helpers belong here too when they only manipulate workbench
  state structure rather than rendering.
  Node/member editing commands should move here as they become pure enough.
  This now includes both 2D and 3D truss editing commands.
  Plane-element editing commands should follow the same pattern.

Keep these modules UI-independent when possible so the workbench surface stays
focused on interaction and presentation.

If code can be reused outside JSX render paths, it should usually prefer this
directory over a large component file.

See:

- [docs/frontend-implementation.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-implementation.md)
