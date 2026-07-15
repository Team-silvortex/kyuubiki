# Documentation System

Use this page as the system-level topology for Kyuubiki documentation.

The goal is to keep the documentation useful for three readers at once:

- humans trying to understand the project
- maintainers deciding where to update behavior
- large-model agents entering the repository without private chat context

## Source Layers

### Central entrypoints

- `README.md`
  Complete local inventory and role-based reading paths.
- `book.html`
  Narrative book entry for humans and assistants.
- `book-manifest.json`
  Machine-readable book map and reading-path manifest.
- `current-line.md`
  Product-line posture and current handoff direction.
- `moxi-handoff.md`
  Final `moxi 2.x` patch gate before `moxi 2.0.0`.

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
- Hub shelf pages under `apps/hub-gui/ui/docs/`

When generated docs drift, change the source JSON/generator first.

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

## Moxi Closeout Posture

During the final `moxi 2.x` patch, documentation changes should close one
of these gaps:

- make the `moxi 2.0.0` handoff clearer
- turn hidden operational knowledge into a source document
- link a capability to its test, evidence, or limitation
- reduce duplicate source-of-truth narratives
- label work as ready, active, watch, or deferred

If a documentation change does none of these, it probably belongs in a later
`2.x` cleanup rather than the final `1.x` patch.
