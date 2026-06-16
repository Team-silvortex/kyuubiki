# Documentation Map

Use this directory as the current source of truth for product shape, protocol
boundaries, deployment modes, and engineering workflow.

## Ownership

- `docs/`
  is the repository-level source of truth for engineering, architecture,
  protocol, deployment, and product-shape documents.
- `apps/hub-gui/ui/docs/`
  is the desktop-facing HTML shelf used inside Hub for operator-facing reading.
  Keep it aligned with this directory, but do not treat the Hub copy as the
  deeper source narrative by default.

## Start Here

Read these first, in order:

1. `book.html`
2. `book-manifest.json`
3. `current-line.md`
4. `system-overview.md`
5. `protocols.md`
6. `repository-structure.md`
7. `testing-and-ci.md`
8. `maintenance.md`

## Central Book

- `book.html`
  Single-entry HTML book for humans and assistants that need the whole project
  picture before diving into chapter-level documents.
- `book-manifest.json`
  Machine-readable chapter map and reading-path manifest for future large-model
  ingestion and tooling.
- `book-ch*.html`
  Chapter pages for the centralized book, so the book can grow by section
  without turning the main entrypoint back into one oversized page.
- `../apps/hub-gui/ui/docs/index.html`
  Desktop-facing mirror entry for the same narrative, tuned for the Hub shelf.

## Maintenance

- `maintenance.md`
  Curation rules for deciding which docs are source-of-truth, planning-only,
  generated, or desktop-facing mirrors.

Then branch by intent:

- `tamamono-minor-lines.md`
  Long-range `tamamono 1.x` roadmap.
- `fem-blender-roadmap.md`
  Product north star and staged path toward becoming the Blender of FEM.
- `rendering-roadmap.html`
  Render-capability upgrade path from the current SVG viewport toward a
  dedicated simulation field renderer.
- `ui-automation-contract.html`
  Product-owned DOM automation contract for the built-in workbench shell.
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
- `fem-blender-roadmap.md`
  Product north star linking workbench, workflow, SDK, and operator-ecosystem growth.

## By Goal

### Understand the system

- `current-line.md`
  Single-entry explanation of the current product line, boundaries, and
  release posture.
- `system-overview.md`
  Full runtime map across GUI, control plane, and solver data plane.
- `app-runtime-boundaries.md`
  Hard role split between Hub, Workbench, Installer, and runtime/agent layers.
- `agent-orchestrator-boundary.md`
  Runtime-side contract for keeping Rust agents, Elixir orchestration, and UI shells decoupled.
  Also includes the current transitional inventory and convergence checklist.
- `headless-agent-contract.md`
  Stable runtime contract for headless Rust agents, including solver-RPC
  authority, descriptor expectations, and the boundary between runtime
  protocols and frontend-owned gateways.
- `architecture-red-lines.md`
  Practical do-not-cross checklist for keeping product and runtime layers clean.
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
- `headless-agent-contract.md`
  Practical runtime-side contract for solver agents, SDK callers, and
  orchestrators that need a cleaner boundary than frontend gateway flows.
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
- `release-prep-1.8.md`
  Practical upgrade checklist and audit entrypoint for moving the repository
  from the current `1.7.x` shipping point toward `1.8.x`.
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
- `development.md`
  Day-to-day contributor workflow, launcher choices, current priorities, and
  the test-first loop for Elixir, Rust, and shared contract work.
- `frontend-style.md`
  Workbench visual language, layout rules, and interaction feel.
- `frontend-implementation.md`
  Frontend component boundaries, state placement, naming, and performance rules.
- `ui-automation-contract.html`
  Human-readable contract for stable automation selectors used by wasm Python
  and headless control tooling.
- `ui-automation-contract.json`
  Machine-readable selector map and rules for the built-in workbench UI shell.
- `language-packs.md`
  Local-first language-pack format, override behavior, and the future remote
  delivery shape for Workbench UI translation packs.
- `fem-blender-roadmap.md`
  Long-range product shape for turning Kyuubiki into a full FEM creation platform.
- `rendering-roadmap.html`
  Concrete rendering-capability staging for `1.6.x` through later `1.x`
  minors, including what can ship before a dedicated field renderer exists.

### Operate and deploy

- `version-line.md`
  Current product-line anchor and version policy.
- `../releases/README.md`
  Lightweight release-snapshot registry for shipped or staged product points.
- `operations.md`
  Deployment modes, watchdog knobs, and runtime entry points.
- `security.md`
  Current guardrails, token protection, and deployment safety notes.
- `packaging-and-deployment.md`
  Component build commands, artifact paths, and packaging output boundaries.
- `desktop-release-checklist.md`
  Platform-specific desktop bundle targets, icon requirements, and release
  checks.
- `update-catalog.html`
  Generated operator-facing view of the unified update tags, release channels,
  and shipped version bindings.
- `rendering-roadmap.html`
  Human-facing staging map for viewport limits, field-render priorities, and
  future multi-physics visualization work.

### Directory entrypoints

- `scripts/README.md`
  Unified launcher, host-side workflow commands, and packaging entrypoints.
- `deploy/README.md`
  Deployment descriptors, agent manifests, and workload-catalog examples.
- `tmp/README.md`
  Disposable local runtime state, working data, and dev-only paths.
- `schemas/README.md`
  Shared JSON contract map for jobs, projects, materials, catalogs, language
  packs, and workflow graphs.

## Suggested Reading Paths

- Browser/frontend work:
  `philosophy.md` -> `frontend-style.md` -> `frontend-implementation.md` -> `ui-automation-contract.html` -> `rendering-roadmap.html`
- Desktop product boundary work:
  `system-overview.md` -> `app-runtime-boundaries.md` -> `agent-orchestrator-boundary.md` -> `architecture-red-lines.md` -> `hub-architecture.md` -> `repository-structure.md`
- Orchestrator/control-plane work:
  `philosophy.md` -> `system-overview.md` -> `agent-orchestrator-boundary.md` -> `operations.md`
- Rust solver/agent work:
  `philosophy.md` -> `system-overview.md` -> `agent-orchestrator-boundary.md` -> `protocols.md`
- Packaging/release work:
  `packaging-and-deployment.md` -> `desktop-release-checklist.md` -> `update-catalog.html`
