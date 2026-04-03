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

Keep these modules UI-independent when possible so the workbench surface stays
focused on interaction and presentation.
