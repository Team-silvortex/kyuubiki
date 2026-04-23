# Documentation Map

Use this directory as the source of truth for repository shape, protocol
boundaries, deployment modes, and day-to-day engineering workflow.

## Start Here

If you are new to the repo, read these first:

1. `system-overview.md`
2. `architecture.md`
3. `protocols.md`
4. `repository-structure.md`
5. `testing-and-ci.md`

## By Goal

### Understand the system

- `system-overview.md`
  Full runtime map across GUI, control plane, and solver data plane.
- `architecture.md`
  Runtime-layer split across frontend, orchestrator, and Rust data plane.
- `philosophy.md`
  Shared product, engineering, and review principles across all layers.
- `repository-structure.md`
  Directory ownership, generated-path boundaries, and source-of-truth rules.

### Build against stable boundaries

- `protocols.md`
  Public HTTP and TCP contracts that let the GUI, control plane, and solver
  agents run as separate programs.
- `headless-sdks.md`
  Protocol-first Rust, Elixir, and Python SDK layer for headless AI and
  automation clients.

### Verify changes

- `testing-and-ci.md`
  Test-layer map, local verification entry points, and CI job layout.
- `tdd.md`
  Test-first expectations across Elixir, Rust, and shared contract work.
- `development.md`
  Day-to-day development conventions, launch modes, and current priorities.
- `frontend-style.md`
  Workbench UI direction, layout rules, and implementation guidance.
- `frontend-implementation.md`
  Component boundaries, state placement, naming, and performance rules for the
  workbench codebase.

### Operate and deploy

- `operations.md`
  Deployment modes, watchdog knobs, and runtime entry points.
- `security.md`
  Current guardrails, token protection, and deployment safety notes.
- `security-sensitive-modules.md`
  Marked source paths that require security-focused review before changes.
- `packaging-and-deployment.md`
  Component build commands, artifact paths, and packaging output boundaries.
- `desktop-release-checklist.md`
  Platform-specific desktop bundle targets, icon requirements, and release
  checks.

## Suggested Reading Paths

- Browser/frontend work:
  `philosophy.md` -> `frontend-style.md` -> `frontend-implementation.md`
- Orchestrator/control-plane work:
  `philosophy.md` -> `architecture.md` -> `operations.md`
- Rust solver/agent work:
  `philosophy.md` -> `architecture.md` -> `protocols.md`
- Packaging/release work:
  `packaging-and-deployment.md` -> `desktop-release-checklist.md`
