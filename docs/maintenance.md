# Documentation Maintenance Guide

Use this note when deciding whether a document should be edited, mirrored,
generated, merged, or eventually archived.

## Document classes

### Source-of-truth docs

These are repository-owned narrative documents that should be edited directly
when product or engineering behavior changes.

Examples:

- `current-line.md`
- `system-overview.md`
- `protocols.md`
- `repository-structure.md`
- `operations.md`
- `testing-and-ci.md`
- `packaging-and-deployment.md`

### Planning docs

These describe direction, not guaranteed present behavior. Keep them, but make
their status obvious and avoid treating them like implementation truth.

Examples:

- `fem-blender-roadmap.md`
- `tamamono-minor-lines.md`
- `accuracy-plan.md`
- `rendering-roadmap.html`

### Baseline and contract docs

These define verification targets or stable product/runtime boundaries. Keep
them synchronized with tests, schemas, and runtime behavior.

Examples:

- `accuracy-baselines.md`
- `workflow-dataset.md`
- `workflow-graph.md`
- `operator-sdk.md`
- `ui-automation-contract.html`
- `ui-automation-contract.json`

### Generated docs

These should not become the main editing surface. Prefer changing the source
JSON or generator, then regenerating the HTML.

Examples:

- `installation-integrity-contract.html`
- `update-catalog.html`

### Desktop shelf docs

These live under `apps/hub-gui/ui/docs/`. They are desktop-facing operator
reading surfaces, not the deeper repository narrative source.

## Curation rules

1. If two docs cover the same thing, keep one as the deeper source and reduce
   the other to a pointer, summary, or generated mirror.
2. If a page is generated, document the generator before editing the HTML by
   hand.
3. If a planning doc starts describing shipped behavior, either tighten its
   status language or move the shipped parts into a source-of-truth doc.
4. If a runtime or protocol change lands, update source-of-truth docs before
   polishing mirrors or shelf pages.
5. If a doc stops matching the current version line, either refresh it in the
   same change or mark it explicitly stale.
6. If a new `docs/*.md/html/json` file is added, add it to `docs/README.md` in
   the same patch.
7. If a new Hub shelf page is added under `apps/hub-gui/ui/docs/`, add it to
   that directory's `README.md` in the same patch.

Run `make check-doc-inventory` to catch invisible local docs before review.

## Current overlap watchlist

- `current-line.md` vs `version-line.md`
  Keep `current-line.md` as the broader narrative and `version-line.md` as the
  short formal release-line note.
- `system-overview.md` vs `app-runtime-boundaries.md` vs
  `agent-orchestrator-boundary.md` vs `headless-agent-contract.md` vs
  `agent-control-authority.md` vs `operations.md` vs
  `installer-remote-control.md`
  Use `runtime-doc-ownership.md` as the primary ownership map so runtime,
  authority, mesh, and remote-control edits do not duplicate whole sections
  across several files.
- `operations.md` vs `packaging-and-deployment.md` vs `desktop-release-checklist.md`
  Keep runtime behavior in `operations.md`, build/output mechanics in
  `packaging-and-deployment.md`, and release steps in
  `desktop-release-checklist.md`.
- `docs/` vs `apps/hub-gui/ui/docs/`
  Keep repository detail in `docs/` and shorter operator-facing reading in the
  Hub shelf.
