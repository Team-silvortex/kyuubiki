# Kyuubiki

Kyuubiki is an engine-first FEM workstation, workflow system, and distributed
runtime control plane.

The active repository line is `moxi 2.x`. The current development snapshot is
`moxi 2.0.0`, the first formal Kyuubiki 2.x line after the `tamamono 1.x`
industrialization bridge.

## What This Repo Contains

- `apps/`
  Browser Workbench, Elixir control plane, and Tauri desktop shells.
- `workers/`
  Rust solver/runtime crates, agent binaries, installer logic, and benchmarks.
- `sdks/`
  Rust, Python, and Elixir headless SDK surfaces.
- `schemas/`
  Shared JSON contracts for workflows, materials, packages, and release assets.
- `deploy/`
  Deployment, update, agent, and install/integrity descriptors.
- `docs/`
  Source-of-truth architecture, operations, verification, and release docs.
- `scripts/`
  Host-facing operational launchers and compatibility wrappers.

## System Shape

Kyuubiki is intentionally split into product, control, and data-plane layers:

- `Workbench`
  Project modeling, workflow authoring, material studies, and result review.
- `Hub`
  Desktop entry shell for runtime posture, project launch, docs, and operator
  guidance.
- `Installer`
  Runtime deployment, repair, update, packaging, and remote-node preparation.
- `Orchestra`
  Elixir control plane for jobs, persistence, chunked results, and scheduling.
- `Agent`
  Rust solver/runtime host with one local engine instance per running agent.
- `SDKs`
  Headless automation surfaces for CI, AI agents, and research scripts.

Supported runtime postures are documented rather than implied:

- `local workstation`: frontend, control plane, and local Rust agents
- `cloud control plane`: frontend/control plane backed by PostgreSQL
- `distributed control plane`: control plane scheduling remote solver nodes
- `direct mesh`: offline or LAN peer mode without ambiguous multi-orchestra
  authority

Start with:

- [docs/current-line.md](docs/current-line.md)
- [docs/current-architecture-map.md](docs/current-architecture-map.md)
- [docs/system-overview.md](docs/system-overview.md)
- [docs/app-runtime-boundaries.md](docs/app-runtime-boundaries.md)
- [docs/agent-control-authority.md](docs/agent-control-authority.md)

## Moxi Baseline

`moxi 2.0.0` is not a reset. It carries forward the stabilized `tamamono 1.x`
contracts and makes them the baseline for the first formal 2.x product line.

The 2.x rule is:

`preserve the engine contracts, prove the computations, and keep UI/runtime
authority separated`.

Use these gates when deciding whether a new 2.x change is safe to treat as
part of the industrial baseline:

- [docs/commercial-readiness-2.0.md](docs/commercial-readiness-2.0.md)
- [docs/minimal-industrial-closure.md](docs/minimal-industrial-closure.md)
- [docs/weakness-roadmap.md](docs/weakness-roadmap.md)
- [docs/moxi-handoff.md](docs/moxi-handoff.md)

## Documentation Entrypoints

Use the book and manifest when you need the whole picture:

- [docs/book.html](docs/book.html)
- [docs/book-manifest.json](docs/book-manifest.json)
- [docs/navigation-matrix.html](docs/navigation-matrix.html)
- [docs/README.md](docs/README.md)
- [docs/documentation-system.md](docs/documentation-system.md)

Read by job:

- Architecture: [docs/module-architecture.md](docs/module-architecture.md),
  [docs/project-architecture-organization.md](docs/project-architecture-organization.md)
- Runtime authority: [docs/agent-orchestrator-boundary.md](docs/agent-orchestrator-boundary.md),
  [docs/headless-agent-contract.md](docs/headless-agent-contract.md)
- Workflows: [docs/workflow-graph.md](docs/workflow-graph.md),
  [docs/workflow-dataset.md](docs/workflow-dataset.md)
- Operators: [docs/operator-sdk.md](docs/operator-sdk.md),
  [docs/operator-task-ir-digest.md](docs/operator-task-ir-digest.md)
- Material research:
  [docs/automated-material-research-example.md](docs/automated-material-research-example.md),
  [docs/material-research-roadmap.md](docs/material-research-roadmap.md)
- Verification: [docs/testing-and-ci.md](docs/testing-and-ci.md),
  [docs/accuracy-baselines.md](docs/accuracy-baselines.md)
- Security and operations: [docs/security.md](docs/security.md),
  [docs/operations.md](docs/operations.md)
- Packaging: [docs/packaging-and-deployment.md](docs/packaging-and-deployment.md),
  [docs/desktop-release-checklist.md](docs/desktop-release-checklist.md)

## Quick Commands

Local development:

```sh
make hot-local
make hot-cloud
make hot-distributed
```

Focused checks:

```sh
make check-version-line
make check-doc-inventory
make check-doc-book
make check-commercial-readiness
make check-module-function-coverage-tensor
make check-operator-reliability
```

Security and reliability:

```sh
make audit-dependencies
make fuzz-smoke
make check-install-update-disk-hygiene
make check-component-integrity-protocol
```

Broader validation:

```sh
make test-web
make test-rust
make test-frontend
make test-sdk
make test-integration
make verify
```

Benchmark and lab evidence:

```sh
make benchmark-profile-plan PROFILE=500k SHAPES=1
make benchmark-profile-remote PROFILE=500k MATRIX=thermal-core CASE=heat-plane-quad-500k
make benchmark-profile-index
```

## Current Capability Posture

Kyuubiki already covers broad FEM and adjacent simulation surfaces at varying
trust levels:

- structural mechanics, truss/frame/beam/plane studies
- thermal and thermo-mechanical studies
- electrostatic and magnetostatic field studies
- acoustic, modal, transport, simplified Stokes, nonlinear, and contact paths
- composite workflow and material research prototypes
- headless Rust-led material study workflows with retained evidence bundles
- 500k/1m exploratory benchmark lanes on the shared lab host

Do not infer equal maturity from equal visibility. The reliability posture is
tracked in:

- [docs/physics-coverage-map.md](docs/physics-coverage-map.md)
- [docs/operator-reliability.md](docs/operator-reliability.md)
- [docs/accuracy-plan.md](docs/accuracy-plan.md)
- [docs/testing-and-ci.md](docs/testing-and-ci.md)

## Deployment Notes

Local defaults are intentionally lightweight:

```sh
KYUUBIKI_DEPLOYMENT_MODE=local
KYUUBIKI_AGENT_DISCOVERY=static
KYUUBIKI_STORAGE_BACKEND=sqlite
SQLITE_DATABASE_PATH=./tmp/data/kyuubiki_dev.sqlite3
```

Cloud and distributed deployments should use explicit runtime configuration
and untracked secret storage. Do not commit real `DATABASE_URL`, SSH
credentials, tokens, private keys, or server-local configuration.

Deployment and update details live in:

- [docs/operations.md](docs/operations.md)
- [docs/installer-remote-control.md](docs/installer-remote-control.md)
- [docs/packaging-and-deployment.md](docs/packaging-and-deployment.md)
- [docs/security.md](docs/security.md)
- [releases/README.md](releases/README.md)

## Repository Rules

- Keep source files under the current `800` line ceiling.
- Keep docs under the current `2000` line ceiling.
- Prefer native Rust operational checks over new shell or Node scripts when the
  work is cross-platform and long-lived.
- Keep GUI, SDK, and runtime semantics aligned; UI convenience must not become
  hidden runtime meaning.
- Keep generated or mirrored docs secondary to source-of-truth documents.
- Keep real secrets out of tracked files.

Before a handoff or large patch, run:

```sh
make check-version-line
make check-doc-inventory
make audit-project-organization
make audit-dependencies
git diff --check
```
