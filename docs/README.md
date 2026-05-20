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

If you want the current release story first:

- `release-support-matrix-0.9.0.md`
- `release-study-coverage-0.9.0.md`
- `release-smoke-matrix-0.9.0.md`
- `release-first-run-0.9.0.md`
- `release-troubleshooting-0.9.0.md`
- `release-readiness-0.9.0.md`
- `release-decision-0.9.0.md`
- `release-github-0.9.0.md`
- `release-notes-0.9.0.md`
- `release-summary-0.9.0.md`

If you want the `v1.0.0` quality direction first:

- `accuracy-plan-1.0.0.md`
- `accuracy-baselines-1.0.0.md`

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
- `accuracy-plan-1.0.0.md`
  Accuracy-validation plan for `v1.0.0`, with benchmark targets, tolerances,
  and automation priorities by study family.
- `accuracy-baselines-1.0.0.md`
  First concrete numerical baseline set for `v1.0.0`, starting with axial,
  thermal-bar, and beam seed cases.
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

- `release-support-matrix-0.9.0.md`
  Official first-release study-family support boundary for `v0.9.0`.
- `release-study-coverage-0.9.0.md`
  Per-study first-run path for samples, orchestrated/direct-mesh usage, and
  report/export expectations.
- `release-smoke-matrix-0.9.0.md`
  Per-study release validation grid for sample open, orchestrated run,
  direct-mesh support, report usability, and export usability.
- `release-first-run-0.9.0.md`
  Recommended first operator session through Hub, workload sync, sample open,
  and orchestrated Workbench run.
- `release-troubleshooting-0.9.0.md`
  First-line “where to look” note for runtime watch, Workbench runtime checks,
  and control-plane health during `v0.9.0` release validation.
- `release-readiness-0.9.0.md`
  Must/should/later checklist for deciding whether `v0.9.0` is ready for an
  initial usable release.
- `release-decision-0.9.0.md`
  Short release-call summary for `v0.9.0`, including current status,
  warnings, blockers, and the recommended ship call.
- `release-github-0.9.0.md`
  GitHub Release-ready Markdown summary for `v0.9.0`.
- `release-summary-0.9.0.md`
  Short external-facing summary for `v0.9.0`, suitable for release copy or
  public update notes.
- `release-notes-0.9.0.md`
  Current release baseline, shipped capability map, and recommended validation
  pass for `v0.9.0`.
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
