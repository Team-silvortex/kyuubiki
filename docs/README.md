# Documentation Map

Use this directory as the source of truth for repository shape and active
engineering direction.

- `architecture.md`
  Runtime-layer split across frontend, orchestrator, and Rust data plane.
- `system-overview.md`
  The full runtime map across GUI, control plane, and solver data plane.
- `protocols.md`
  Public HTTP and TCP contracts that let the GUI, control plane, and solver agents run as separate programs.
- `headless-sdks.md`
  Protocol-first Rust, Elixir, and Python SDK layer for headless AI and automation clients.
- `testing-and-ci.md`
  Test-layer map, local verification entry points, and CI job layout.
- `operations.md`
  Deployment modes, watchdog knobs, and runtime entry points.
- `packaging-and-deployment.md`
  Component build commands, artifact paths, and packaging output boundaries.
- `desktop-release-checklist.md`
  Platform-specific desktop bundle targets, icon requirements, and release checks.
- `security.md`
  Current guardrails, token protection, and deployment safety notes.
- `repository-structure.md`
  Concrete directory ownership and generated-path boundaries.
- `development.md`
  Day-to-day development conventions, launch modes, and current priorities.
- `tdd.md`
  TDD workflow and test-first expectations across Elixir and Rust.

Read these in roughly this order if you are new to the repository:

1. `system-overview.md`
2. `architecture.md`
3. `protocols.md`
4. `headless-sdks.md`
5. `testing-and-ci.md`
6. `security.md`
7. `operations.md`
8. `packaging-and-deployment.md`
9. `desktop-release-checklist.md`
10. `repository-structure.md`
11. `development.md`
