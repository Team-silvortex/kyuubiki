# Documentation Map

Use this directory as the current source of truth for product shape, protocol
boundaries, deployment modes, and engineering workflow.

## Start Here

Read these first, in order:

1. `current-line.md`
2. `system-overview.md`
3. `protocols.md`
4. `repository-structure.md`
5. `testing-and-ci.md`

Then branch by intent:

- `tamamono-minor-lines.md`
  Long-range `tamamono 1.x` roadmap.
- `accuracy-plan.md`
  Quality direction and verification policy.
- `accuracy-baselines.md`
  Current numerical baselines.
- `operator-sdk.md`
  Current extension-contract direction for new operator capabilities.
- `workflow-graph.md`
  Multi-operator composition model for shader-like workflow growth.
- `workflow-dataset.md`
  ONNX-like cross-operator data contract for workflow-carried values.

## By Goal

### Understand the system

- `current-line.md`
  Single-entry explanation of the current product line, boundaries, and
  release posture.
- `system-overview.md`
  Full runtime map across GUI, control plane, and solver data plane.
- `philosophy.md`
  Shared product, engineering, and review principles across all layers.
- `hub-architecture.md`
  Hub-specific entrypoint and operator-shell rationale.
- `repository-structure.md`
  Directory ownership, generated-path boundaries, and source-of-truth rules.

### Build against stable boundaries

- `protocols.md`
  Public HTTP and TCP contracts that let the GUI, control plane, and solver
  agents run as separate programs.
- `headless-sdks.md`
  Protocol-first Rust, Elixir, and Python SDK layer for headless AI and
  automation clients.
- `operator-sdk.md`
  Proposed operator descriptor, runtime contract, and validation shape for
  future extensible solver and transform capabilities.
- `workflow-graph.md`
  Proposed graph/runtime model for chaining multiple operators into stable
  headless workflows.
- `workflow-dataset.md`
  Portable dataset/value contract shared across workflow nodes, ports, and
  operator schemas.

### Verify changes

- `version-line.md`
  Current version-line note for `tamamono 1.x`, including codename and major
  version policy.
- `tamamono-minor-lines.md`
  Suggested long-range grouping for the `tamamono 1.x` minor releases.
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
  `axial_bar_1d`, `thermal_bar_1d`, `frame_3d`, `truss_2d`,
  `thermal_frame_3d`, and `thermal_truss_3d`.
- `tdd.md`
  Test-first expectations across Elixir, Rust, and shared contract work.
- `development.md`
  Day-to-day contributor workflow, launcher choices, and current priorities.
- `frontend-style.md`
  Workbench visual language, layout rules, and interaction feel.
- `frontend-implementation.md`
  Frontend component boundaries, state placement, naming, and performance rules.
- `language-packs.md`
  Local-first language-pack format, override behavior, and the future remote
  delivery shape for Workbench UI translation packs.

### Operate and deploy

- `version-line.md`
  Current product-line anchor and version policy.
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

### Directory entrypoints

- `/Users/Shared/chroot/dev/kyuubiki/scripts/README.md`
  Unified launcher, host-side workflow commands, and packaging entrypoints.
- `/Users/Shared/chroot/dev/kyuubiki/deploy/README.md`
  Deployment descriptors, agent manifests, and workload-catalog examples.
- `/Users/Shared/chroot/dev/kyuubiki/dist/README.md`
  Generated desktop artifacts, build summaries, and release-output layout.
- `/Users/Shared/chroot/dev/kyuubiki/tmp/README.md`
  Disposable local runtime state, working data, and dev-only paths.
- `/Users/Shared/chroot/dev/kyuubiki/schemas/README.md`
  Shared JSON contract map for jobs, projects, materials, catalogs, language
  packs, and workflow graphs.

## Suggested Reading Paths

- Browser/frontend work:
  `philosophy.md` -> `frontend-style.md` -> `frontend-implementation.md`
- Orchestrator/control-plane work:
  `philosophy.md` -> `system-overview.md` -> `operations.md`
- Rust solver/agent work:
  `philosophy.md` -> `system-overview.md` -> `protocols.md`
- Packaging/release work:
  `packaging-and-deployment.md` -> `desktop-release-checklist.md`
