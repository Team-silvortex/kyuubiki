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
3. `navigation-matrix.html`
4. `current-line.md`
5. `system-overview.md`
6. `module-architecture.md`
7. `project-architecture-organization.md`
8. `protocols.md`
9. `repository-structure.md`
10. `testing-and-ci.md`
11. `maintenance.md`
12. `installer-remote-control.md`
13. `minimal-industrial-closure.md`

For the current `1.15.x` hardening path, keep four threads mentally linked:

- centralized docs book and Hub shelf mirrors
- headless live execution checks
- Installer-owned remote control
- orchestrated and direct-mesh runtime posture

## Central Book

- `book.html`
  Single-entry HTML book for humans and assistants that need the whole project
  picture before diving into chapter-level documents.
- `book-manifest.json`
  Machine-readable chapter map and reading-path manifest for future large-model
  ingestion and tooling.
- `navigation-matrix.html`
  Cross-cutting role and lane matrix that ties the book, verification, remote
  control, mesh posture, and headless SDK paths together.
- `book-ch*.html`
  Chapter pages for the centralized book, so the book can grow by section
  without turning the main entrypoint back into one oversized page.
- `../apps/hub-gui/ui/docs/index.html`
  Desktop-facing mirror entry for the same narrative, tuned for the Hub shelf.

## Maintenance

- `maintenance.md`
  Curation rules for deciding which docs are source-of-truth, planning-only,
  generated, or desktop-facing mirrors.
- `runtime-doc-ownership.md`
  Runtime, authority, mesh, and Installer remote-control documentation
  ownership map for avoiding duplicate narrative drift.

Then branch by intent:

- `tamamono-minor-lines.md`
  Long-range `tamamono 1.x` roadmap.
- `commercial-readiness-2.0.md`
  Trust-gate checklist for deciding whether `2.0` can honestly ship as an
  early-commercial / research-partner line.
- `commercial-readiness-2.0.manifest.json`
  Machine-readable gate map for the same `2.0` checklist, validated by
  `node ./scripts/validate-commercial-readiness.mjs`.
- `minimal-industrial-closure.md`
  The narrower `1.15.x -> 1.20.x` bridge checklist for closing the first
  bounded industrial workflow before broader `2.0` commercial claims.
- `module-architecture.md`
  System-wide module map for product shells, control plane, runtime data
  plane, SDKs, contracts, verification gates, and where new work belongs.
- `minimal-industrial-closure.manifest.json`
  Machine-readable gate map for the same minimum industrial loop, validated by
  `node ./scripts/validate-minimal-industrial-closure.mjs`.
- `fem-blender-roadmap.md`
  Product north star and staged path toward becoming the Blender of FEM,
  including the `2.0` early-commercial trust line and `3.0`
  giant-challenge line.
- `rendering-roadmap.html`
  Render-capability upgrade path from the current SVG viewport toward a
  dedicated simulation field renderer.
- `ui-architecture-migration.md`
  UI refactor tracker for moving shared desktop and app-specific JavaScript
  into typed contracts without destabilizing Tauri packaging.
- `ui-automation-contract.html`
  Product-owned DOM automation contract for the built-in workbench shell.
- `accuracy-plan.md`
  Quality direction and verification policy.
- `accuracy-baselines.md`
  Current numerical baselines.
- `solver-matrix-optimization-pack.md`
  Benchmark-backed note for the retained Rust solver matrix optimizations and
  the experiments that should not be repeated blindly.
- `operator-sdk.md`
  Current extension-contract direction for new operator capabilities.
- `operator-reliability.md`
  Machine-readable reliability manifest policy for solve operators, including
  trust levels, evidence expectations, and the current release minimum coverage
  gate.
- `material-research-roadmap.md`
  Reliability roadmap for moving material studies from runnable prototypes to
  reproducible screening, review, and eventual qualification workflows.
- `material-score-contract.md`
  Cross-runtime result contract for material candidate scoring, including
  criteria, ranges, feasibility policy, ranking semantics, and stable errors.
- `physics-coverage-map.md`
  `1.15.x` coverage map for broad solver-family execution coverage and the
  review-level reliability gate before the `1.15.x` and `1.16.x`
  engine/task-format contract freeze.
- `workflow-graph.md`
  Multi-operator composition model for shader-like workflow growth.
- `workflow-dataset.md`
  ONNX-like cross-operator data contract for workflow-carried values.
- `installer-remote-control.md`
  Installer-owned remote deployment, certificate, mesh, and workflow-snapshot
  control-surface note for the `1.15.x` preparation line.
- `remote-deployment-roadmap.html`
  Maturity roadmap for moving Installer remote deployment from policy-bounded
  SSH transport to a deployment service with plans, journals, artifact
  delivery, host trust, and tests.
- `component-integrity-protocol.html`
  Component ownership, required-path coverage, protection, cleanup, and visible
  rule protocol used by the Installer integrity report.
- `remote-pilot.md`
  Practical first-Ubuntu-host rollout sketch for introducing remote solver
  nodes, then remote control-plane and workload-source validation.
- `fem-blender-roadmap.md`
  Product north star linking workbench, workflow, SDK, and operator-ecosystem growth.

## By Goal

### Understand the system

- `current-line.md`
  Single-entry explanation of the current product line, boundaries, and
  release posture.
- `system-overview.md`
  Full runtime map across GUI, control plane, and solver data plane.
- `module-architecture.md`
  First architecture map for understanding which module owns a capability and
  how the product, control plane, runtime, SDK, contract, and verification
  layers fit together.
- `app-runtime-boundaries.md`
  Hard role split between Hub, Workbench, Installer, and runtime/agent layers.
- `mobile-gui-runtime-boundary.md`
  Mobile WebView GUI contract: remote-control only, no local agent/orchestra
  hosting.
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
- `project-architecture-organization.md`
  Current `1.15.x` architecture organization map for Hub, Workbench,
  Installer, Orchestra, Agent, SDKs, schemas, TaskIR, and 600-line cleanup
  boundaries.
- `minimal-industrial-closure.md`
  Current bounded industrial-loop closure map tying executable TaskIR,
  operator reliability, agent/orchestra, installer, persistence, security,
  UX, and benchmark gates together.

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
- `installer-remote-control.md`
  Installer-side remote deployment and runtime-control contract for the
  operator-facing remote node surface.
- `remote-pilot.md`
  Operator rollout sketch for bringing up the first Ubuntu-hosted remote target
  without overloading `operations.md`.

### Verify changes

- `version-line.md`
  Current version-line note for `tamamono 1.x`, including codename and major
  version policy.
- `release-prep-1.9-to-1.20.md`
  Industrialization roadmap for the second half of `tamamono 1.x`, covering
  the boundary-hardening path from `1.15.x` through `1.20.x` before `2.0`,
  plus the commercial distinction between `2.0`, `2.x`, and `3.0`.
- `commercial-readiness-2.0.md`
  Commercial trust-gate checklist for numerical confidence, workflow assets,
  SDK credibility, installer/update hygiene, agent authority, security, UX, and
  docs readiness before `2.0`.
- `installer-remote-control.md`
  Source-of-truth note for the Installer remote control surface that now sits
  inside the `1.15.x` trust-hardening and asset-formalization path.
- `component-integrity-protocol.html`
  Protocol page for adding new components without leaving required layout,
  brand metadata, protection, or cleanup behavior outside integrity coverage.
- `tamamono-minor-lines.md`
  Suggested long-range grouping for the `tamamono 1.x` minor releases.
- `testing-and-ci.md`
  Test-layer map, local verification entry points, and CI job layout.
- frontend and Rust headless live execution now sit here as first-class
  service-executor validation paths for `service_health`,
  `workflow_submit_catalog`, and `workflow_submit_graph`
- `solver-matrix-optimization-pack.md`
  Current matrix-side optimization pack for the Rust solver, with A/B evidence
  and benchmark interpretation notes.
- `development.md`
  Day-to-day contributor workflow, including when workflow-heavy frontend work
  should run the dedicated `workflow-preflight` guard.
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
- `installer-remote-control.md`
  Installer-owned remote node, certificate, mesh, and workflow-snapshot
  control surface for operator-facing deployment work.
- `security.md`
  Current guardrails, token protection, and deployment safety notes.
- `packaging-and-deployment.md`
  Component build commands, artifact paths, and packaging output boundaries.
- `desktop-release-checklist.md`
  Platform-specific desktop bundle targets, icon requirements, and release
  checks.
- `testing-and-ci.md`
  Verification map for repo checks, CI layers, and the dedicated workflow
  preflight path used for workflow-heavy frontend changes.
- `update-catalog.html`
  Generated operator-facing view of the unified update tags, release channels,
  and shipped version bindings.
- `rendering-roadmap.html`
  Human-facing staging map for viewport limits, field-render priorities, and
  future multi-physics visualization work.

### Directory entrypoints

- `scripts/README.md`
  Unified launcher, host-side workflow commands, and packaging entrypoints.
- `../config/README.md`
  Checked-in configuration contracts for reliability gates, qualification
  planning, benchmark coverage, audit lanes, and self-host toolchains.
- `deploy/README.md`
  Deployment descriptors, agent manifests, and workload-catalog examples.
- `tmp/README.md`
  Disposable local runtime state, working data, and dev-only paths.
- `../tmp/nightly-overview.html`
  Human-readable local index for the latest retained nightly regression
  artifacts across direct-mesh, workflow-catalog, and standard benchmark lanes.
- `schemas/README.md`
  Shared JSON contract map for jobs, projects, materials, catalogs, language
  packs, and workflow graphs.

## Suggested Reading Paths

- Browser/frontend work:
  `philosophy.md` -> `frontend-style.md` -> `frontend-implementation.md` -> `ui-automation-contract.html` -> `rendering-roadmap.html`
- Workflow-heavy frontend work:
  `workflow-graph.md` -> `workflow-dataset.md` -> `development.md` -> `testing-and-ci.md` -> `release-prep-1.9-to-1.20.md`
- Installer remote/runtime-control work:
  `system-overview.md` -> `app-runtime-boundaries.md` -> `installer-remote-control.md` -> `security.md` -> `testing-and-ci.md`
- Desktop product boundary work:
  `system-overview.md` -> `project-architecture-organization.md` -> `app-runtime-boundaries.md` -> `agent-orchestrator-boundary.md` -> `architecture-red-lines.md` -> `hub-architecture.md` -> `repository-structure.md`
- Orchestrator/control-plane work:
  `philosophy.md` -> `system-overview.md` -> `project-architecture-organization.md` -> `agent-orchestrator-boundary.md` -> `operations.md`
- Rust solver/agent work:
  `philosophy.md` -> `system-overview.md` -> `project-architecture-organization.md` -> `agent-orchestrator-boundary.md` -> `protocols.md` -> `solver-matrix-optimization-pack.md`
- Material research work:
  `material-research-roadmap.md` -> `headless-sdks.md` -> `accuracy-plan.md` -> `workflow-dataset.md` -> `operator-sdk.md`
- Packaging/release work:
  `packaging-and-deployment.md` -> `testing-and-ci.md` -> `desktop-release-checklist.md` -> `commercial-readiness-2.0.md` -> `update-catalog.html`
