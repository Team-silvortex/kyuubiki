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

Keep these modules UI-independent when possible so the workbench surface stays
focused on interaction and presentation.

If code can be reused outside JSX render paths, it should usually prefer this
directory over a large component file.

See:

- [docs/frontend-implementation.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-implementation.md)
