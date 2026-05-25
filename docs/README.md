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

If you want the current version-line story first:

- `current-line.md`

If you want the preserved initial usable release pack:

- `release-archive-0.9.0.md`

If you want the `v1.x` quality direction first:

- `current-line.md`
- `tamamono-minor-lines.md`
- `accuracy-plan.md`
- `accuracy-baselines.md`
- `version-line.md`

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

- `current-line.md`
  Single-entry current-line guide for `tamamono 1.x`.
- `version-line.md`
  Current version-line note for `tamamono 1.x`, including codename, major
  version policy, and how the line relates to the preserved `0.9.0` release
  pack.
- `tamamono-minor-lines.md`
  Suggested long-range grouping for the `tamamono 1.x` minor releases.
- `release-archive-0.9.0.md`
  Single-entry archive for the full `0.9.0` initial usable release evidence
  pack.
- `testing-and-ci.md`
  Test-layer map, local verification entry points, and CI job layout.
- `accuracy-plan.md`
  Accuracy-validation plan for `v1.x`, with benchmark targets, tolerances,
  and automation priorities by study family.
- `accuracy-baselines.md`
  First concrete numerical baseline set for `v1.x`, starting with axial,
  thermal-bar, and beam seed cases.
- `tests/integration/README.md`
  Cross-process smoke map, including the current orchestrated API path for
  `axial_bar_1d`, `frame_3d`, `thermal_frame_3d`, and `thermal_truss_3d`.
- `tdd.md`
  Test-first expectations across Elixir, Rust, and shared contract work.
- `development.md`
  Day-to-day development conventions, launch modes, and current priorities.
- `frontend-style.md`
  Workbench UI direction, layout rules, and implementation guidance.
- `frontend-implementation.md`
  Component boundaries, state placement, naming, and performance rules for the
  workbench codebase.
- `language-packs.md`
  Local-first language-pack format, override behavior, and the future remote
  delivery shape for Workbench UI translation packs.

### Operate and deploy

- `current-line.md`
  Current product-line starting point before drilling into accuracy or archive details.
- `version-line.md`
  Current product-line anchor before dropping into the preserved release pack.
- `release-archive-0.9.0.md`
  Single-entry archive for the full preserved `v0.9.0` release pack.
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
- `ubuntu24-hub-pilot.md`
  A practical first remote-target pilot guide for using one Ubuntu 24 machine
  as a real Hub runtime and workload test server.

## Suggested Reading Paths

- Browser/frontend work:
  `philosophy.md` -> `frontend-style.md` -> `frontend-implementation.md`
- Orchestrator/control-plane work:
  `philosophy.md` -> `architecture.md` -> `operations.md`
- Rust solver/agent work:
  `philosophy.md` -> `architecture.md` -> `protocols.md`
- Packaging/release work:
  `packaging-and-deployment.md` -> `desktop-release-checklist.md`
