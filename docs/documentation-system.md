# Documentation System

Use this page as the system-level topology for Kyuubiki documentation.

The goal is to keep the documentation useful for three readers at once:

- humans trying to understand the project
- maintainers deciding where to update behavior
- large-model agents entering the repository without private chat context

## Source Layers

### Central entrypoints

- repository `README.md`
  Product overview, boundaries and a short startup route.
- `docs/README.md`
  Complete local inventory grouped by task and responsibility.
- `book.html`
  Narrative book entry for humans and assistants.
- `book-manifest.json`
  Machine-readable book map and reading-path manifest.
- `current-line.md`
  Compact active status, version/evidence ledger and current handoff direction.
- `daji-3.6-to-3.7-history.md`, `daji-3.5-history.md`,
  `daji-3.3-to-3.4-history.md`
  Historical checkpoint narratives with their original evidence scope.
- `moxi-handoff.md`
  Historical gate that defined the transition into `moxi 2.x`.

### Source-of-truth narratives

These documents own durable behavior and should be edited before mirrors:

- architecture: `system-overview.md`, `module-architecture.md`,
  `current-architecture-map.md`, `app-runtime-boundaries.md`
- runtime authority: `agent-control-authority.md`,
  `agent-orchestrator-boundary.md`, `headless-agent-contract.md`
- workflows and assets: `workflow-graph.md`, `workflow-dataset.md`,
  `material-score-contract.md`
- operations: `operations.md`, `packaging-and-deployment.md`,
  `installer-remote-control.md`, `security.md`
- verification: `testing-and-ci.md`, `accuracy-plan.md`,
  `accuracy-baselines.md`, `operator-reliability.md`

### Contract and manifest pairs

Markdown explains intent; JSON or HTML keeps machine/audit surfaces aligned:

- `commercial-readiness-2.0.md` and
  `commercial-readiness-2.0.manifest.json`
- `moxi-handoff.md` and `moxi-handoff.manifest.json`
- `minimal-industrial-closure.md` and
  `minimal-industrial-closure.manifest.json`
- `material-score-contract.md` and `material-score-contract.manifest.json`
- `automated-material-research-example.md` and
  `automated-material-research-example.manifest.json`
- `ui-automation-contract.html` and `ui-automation-contract.json`

### Generated and mirrored pages

Do not treat generated HTML as the primary edit surface:

- `update-catalog.html`
- `installation-integrity-contract.html`

When generated docs drift, change the source JSON/generator first.
Central book chapters and the short Hub shelf narratives are hand-maintained
presentations, not generated behavioral contracts. Edit their topic owner first,
then update those presentations. Keep user-facing reading in HTML rather than
making an operator follow a raw Markdown source as the only route.

## Inventory Rules

Every local `docs/*.md`, `docs/*.html`, and `docs/*.json` file must be visible
from `docs/README.md`.

Every Hub shelf page under `apps/hub-gui/ui/docs/` must be visible from that
directory's `README.md`.

Run:

```sh
make check-doc-inventory
```

This inventory check is intentionally simple. It does not decide whether a
document is good; it ensures no document is invisible.

## Maintenance Rules

1. Add new source docs to `docs/README.md` in the same patch.
2. Add new book-scale docs to `docs/book-manifest.json` when they are part of
   a central reading path.
3. Add new runtime-boundary docs to `runtime-doc-ownership.md` when they overlap
   with agent, orchestra, mesh, installer, or protocol ownership.
4. Add new generated docs to `maintenance.md` with their generator or source
   contract.
5. Add new Hub shelf pages to `apps/hub-gui/ui/docs/README.md`.
6. Avoid absolute local paths, machine-specific commands, or chat-only context
   in source docs.

## Daji Maintenance Posture

During the active `daji 3.x` line, documentation changes should close one of
these gaps:

- separate package/book version, version-named Git checkpoint, recorded
  installation and source working-tree evidence
- turn hidden operational knowledge into a source document
- link a capability to its test, evidence, or limitation
- reduce duplicate source-of-truth narratives
- label work as ready, active, watch, or deferred

If a documentation change does none of these, it probably belongs in a later
cleanup rather than the current release checkpoint.

## Current Status And History

Keep the active page short enough to answer what exists, what changed and what
must be verified next. Move superseded checkpoint descriptions to the matching
history file rather than copying them into book chapters or Hub mirrors.
Preserve their body, dates, relative links and old entry anchors; do not relabel
historical source tests as fresh installed acceptance.

Package labels follow the manifests. Recorded installation labels follow a
specific retained snapshot and must not be advanced by the book-version sync.
Git subjects are a separate ledger field. On any release edit, check all three
identities rather than replacing every version-like string.

The document inventory proves discoverability; the book check proves its
configured HTML paths and metadata. Neither validates the scientific truth of
a report, every Markdown fragment or the current health of a user's machine.
