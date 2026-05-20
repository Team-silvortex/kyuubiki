# kyuubiki tamamono 1.0.0

Kyuubiki is an engine-first FEM workstation and control plane with a browser-first workbench:

- `Next.js` workbench UI for modeling, project management, result review, and immersive 3D editing
- `Elixir` orchestrator API for jobs, persistence, chunked result delivery, and multi-agent coordination
- `Rust` solver agents for FEM data-plane computation, benchmarking, and engine-style reusable core logic
- an emerging `Kyuubiki Hub` desktop shell for project launch, runtime control, and operator-facing orchestration

`tamamono 1.0.0` is the current repository-wide version line.

It also now has an explicit deployment split:

- `local workstation`: frontend + orchestrator + local Rust agents
- `cloud control plane`: frontend + orchestrator + PostgreSQL
- `distributed control plane`: orchestrator/frontend separated from remotely deployed solver nodes

On the desktop side, `Kyuubiki Hub` sits above those runtime shapes. The Hub is
the operator-facing shell; the orchestrator is one managed runtime workload that
the Hub can start locally, connect to remotely, or switch between across
targets.
- `headless peer mesh`: Rust solver agents can run without a GUI or Phoenix on the same host, and can advertise LAN peer-cluster topology through the shared RPC descriptor

For local iteration, the repo-level hot-reload entrypoints are:

- `make hot-local`
- `make hot-cloud`
- `make hot-distributed`

These commands keep Next.js/Tauri HMR where it already exists and add the
missing restart-on-change loop for the non-Phoenix Elixir control plane and
Rust solver agents.

Frontend runtime modes are explicitly split:

- `orchestrated_gui`
  the current workbench mode, where the editor talks to Phoenix and Phoenix
  talks to solver agents
- `direct_mesh_gui`
  a future mode where the editor can operate directly against a LAN peer mesh
  of headless Rust agents without requiring Phoenix on the solver hot path

Protocol boundaries are explicit:

- `kyuubiki.control-plane/http-v1`
- `kyuubiki.solver-rpc/v1`

## Repository Shape

The monorepo is intentionally split by responsibility:

- `apps/`
  Product-facing surfaces such as the browser workbench, orchestrator API, and
  installer GUI
- `workers/`
  Rust data-plane crates, solver runtime, benchmark tooling, and installer CLI
- `sdks/`
  Headless protocol clients for automation, AI agents, and external tools
- `schemas/`
  Versioned JSON contracts shared across the whole stack
- `deploy/`
  Deployment descriptors such as agent manifests
- `docs/`
  Architecture, development, and repository-structure references
- `scripts/`
  Host-native launch and workflow entry points

Start here if you need the repo map:

- [docs/README.md](/Users/Shared/chroot/dev/kyuubiki/docs/README.md)
- [docs/current-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/current-line.md)
- [docs/release-archive-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-archive-0.9.0.md)
- [docs/system-overview.md](/Users/Shared/chroot/dev/kyuubiki/docs/system-overview.md)
- [docs/hub-architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/hub-architecture.md)
- [docs/philosophy.md](/Users/Shared/chroot/dev/kyuubiki/docs/philosophy.md)
- [docs/repository-structure.md](/Users/Shared/chroot/dev/kyuubiki/docs/repository-structure.md)
- [docs/protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md)
- [docs/headless-sdks.md](/Users/Shared/chroot/dev/kyuubiki/docs/headless-sdks.md)
- [docs/testing-and-ci.md](/Users/Shared/chroot/dev/kyuubiki/docs/testing-and-ci.md)
- [docs/accuracy-plan.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-plan.md)
- [docs/accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)
- [docs/frontend-style.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-style.md)
- [docs/frontend-implementation.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-implementation.md)
- [docs/security.md](/Users/Shared/chroot/dev/kyuubiki/docs/security.md)
- [docs/operations.md](/Users/Shared/chroot/dev/kyuubiki/docs/operations.md)
- [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
- [scripts/README.md](/Users/Shared/chroot/dev/kyuubiki/scripts/README.md)

If you want the shortest current-line explanation first, start with
[docs/current-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/current-line.md).

## Read By Intent

- new to the project:
  [docs/philosophy.md](/Users/Shared/chroot/dev/kyuubiki/docs/philosophy.md),
  [docs/system-overview.md](/Users/Shared/chroot/dev/kyuubiki/docs/system-overview.md),
  [docs/architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/architecture.md),
  [docs/repository-structure.md](/Users/Shared/chroot/dev/kyuubiki/docs/repository-structure.md)
- changing protocols or automation surfaces:
  [docs/philosophy.md](/Users/Shared/chroot/dev/kyuubiki/docs/philosophy.md),
  [docs/protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md),
  [docs/headless-sdks.md](/Users/Shared/chroot/dev/kyuubiki/docs/headless-sdks.md)
- changing tests, CI, or release flow:
  [docs/testing-and-ci.md](/Users/Shared/chroot/dev/kyuubiki/docs/testing-and-ci.md),
  [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md),
  [docs/desktop-release-checklist.md](/Users/Shared/chroot/dev/kyuubiki/docs/desktop-release-checklist.md)
- changing workbench UX or visual language:
  [docs/philosophy.md](/Users/Shared/chroot/dev/kyuubiki/docs/philosophy.md),
  [docs/frontend-style.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-style.md),
  [docs/frontend-implementation.md](/Users/Shared/chroot/dev/kyuubiki/docs/frontend-implementation.md),
  [apps/frontend/README.md](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/README.md)

## What tamamono 1.0.0 Can Do

### Supported analysis domains

- `Mechanical`
  axial, spring, beam, torsion, truss, frame, and plane studies
- `Thermal`
  heat-bar and heat-plane studies with temperature and flux review
- `Thermo-mechanical`
  thermal bar, beam, frame, truss, and plane response paths

### Supported study families

Verified baseline:

- `Axial & Springs`
  `axial_bar_1d`, `thermal_bar_1d`, `spring_1d`
- `Beams & Frames`
  `beam_1d`, `torsion_1d`, `frame_2d`
- `Trusses`
  `truss_2d`, `truss_3d`
- `Planes`
  `plane_triangle_2d`, `plane_quad_2d`

Current line coverage:

- `Thermal`
  `heat_bar_1d`, `heat_plane_triangle_2d`, `heat_plane_quad_2d`
- `Thermo-mechanical`
  `thermal_beam_1d`, `thermal_frame_2d`, `thermal_truss_2d`,
  `thermal_truss_3d`, `thermal_plane_triangle_2d`,
  `thermal_plane_quad_2d`

Lighter operator families:

- `spring_2d`
- `spring_3d`

### Industrial work surfaces

- family-aware Workbench study selection across `Mechanical`, `Thermal`, and
  `Thermo-mechanical`
- official sample library grouped by domain and family
- result-aware `Report`, `Tree`, hotspot, and export paths across line and
  plane studies
- direct editing paths for representative `2D truss`, `3D truss`, and `2D frame`
- immersive `3D` workspace tools, large-result chunk browsing, and undo/redo
- material library, multi-material assignment, and viewport material filtering

### Operator and runtime workflow

- `Kyuubiki Hub` as the desktop entry shell
- project bundle inspect / validate / normalize / unpack / pack / diff
- workload library with local attach, remote catalog sync, and open-in-workbench
- runtime watch for stack logs and hot-reload logs
- managed hot-reload loops for `local`, `cloud`, and `distributed` dev shapes
- desktop readiness, staging, host build, and verification flow through CLI and Hub

### Persistence and automation

- project / model / version / job / result persistence
- project bundle import / export and control-plane bundle download
- Python, Elixir, and Rust headless SDKs
- frontend DSL, macro recording, JSON import / export, and project-scoped automation presets
- formal `kyuubiki.workload-catalog/v1` workload schema and catalog-backed distribution

### Current validated workflows

- Workbench sample open, report, and export coverage across `Mechanical`,
  `Thermal`, and `Thermo-mechanical`
- orchestrated and direct-mesh smoke for supported study families
- `heat -> thermo-mechanical` bridge workflows for supported bar and plane paths
- repo-level validation through web, Rust, frontend, SDK, integration, desktop,
  and desktop-status baselines

For the current product-line posture, major-version policy, and quality
direction, use:

- [docs/current-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/current-line.md)
- [docs/version-line.md](/Users/Shared/chroot/dev/kyuubiki/docs/version-line.md)
- [docs/tamamono-minor-lines.md](/Users/Shared/chroot/dev/kyuubiki/docs/tamamono-minor-lines.md)
- [docs/accuracy-plan.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-plan.md)
- [docs/accuracy-baselines.md](/Users/Shared/chroot/dev/kyuubiki/docs/accuracy-baselines.md)
- [docs/release-archive-0.9.0.md](/Users/Shared/chroot/dev/kyuubiki/docs/release-archive-0.9.0.md)

Packaging and deployment paths are now documented centrally in:

- [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)
- [docs/desktop-release-checklist.md](/Users/Shared/chroot/dev/kyuubiki/docs/desktop-release-checklist.md)

Desktop release planning and platform bundle checks now live in the two docs
above rather than in the repo root.

## Architecture and Validation

Current architecture, runtime splits, and contract edges live in:

- [docs/system-overview.md](/Users/Shared/chroot/dev/kyuubiki/docs/system-overview.md)
- [docs/architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/architecture.md)
- [docs/protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md)

Current validation layers and smoke entrypoints live in:

- [docs/testing-and-ci.md](/Users/Shared/chroot/dev/kyuubiki/docs/testing-and-ci.md)
- [tests/integration/README.md](/Users/Shared/chroot/dev/kyuubiki/tests/integration/README.md)

The main repo-level verification entrypoints are:

- `make test-web`
- `make test-rust`
- `make test-frontend`
- `make test-sdk`
- `make test-integration`
- `make test-hub-gui`
- `make test-installer-gui`
- `make test-workbench-gui`
- `make desktop-status PLATFORM=all`
- `make build-workbench-gui`
- `make package-runtime`
- `make package-desktop`

The packaging/output map for those commands lives in:

- [docs/packaging-and-deployment.md](/Users/Shared/chroot/dev/kyuubiki/docs/packaging-and-deployment.md)

## Storage Modes

Kyuubiki supports two SQL-backed storage modes:

- `sqlite`
  Best default for local single-machine work. Minimal host burden.
- `postgres`
  Recommended for cloud, multi-node, and distributed deployments.

Persisted entities include:

- projects
- models
- model versions
- jobs
- results

Recommended local setup:

```bash
KYUUBIKI_DEPLOYMENT_MODE=local
KYUUBIKI_AGENT_DISCOVERY=static
KYUUBIKI_STORAGE_BACKEND=sqlite
SQLITE_DATABASE_PATH=./tmp/data/kyuubiki_dev.sqlite3
KYUUBIKI_AGENT_ENDPOINTS=127.0.0.1:5001,127.0.0.1:5002
```

Recommended cloud/distributed setup:

```bash
KYUUBIKI_DEPLOYMENT_MODE=distributed
KYUUBIKI_AGENT_DISCOVERY=registry
KYUUBIKI_AGENT_MANIFEST_PATH=./deploy/agents.distributed.example.json
KYUUBIKI_STORAGE_BACKEND=postgres
DATABASE_URL=ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev
```

## Security and Guardrails

Kyuubiki now has a small but formal deployment guardrail layer:

- optional control-plane API token protection via:
  - `KYUUBIKI_API_TOKEN`
  - `KYUUBIKI_CLUSTER_API_TOKEN`
  - `KYUUBIKI_CLUSTER_ALLOWED_AGENT_IDS`
  - `KYUUBIKI_CLUSTER_ALLOWED_CLUSTER_IDS`
  - `KYUUBIKI_CLUSTER_REQUIRE_FINGERPRINT`
  - `KYUUBIKI_CLUSTER_TIMESTAMP_WINDOW_MS`
  - `KYUUBIKI_PROTECT_READS`
- optional direct-mesh route protection via:
  - `KYUUBIKI_DIRECT_MESH_ENABLED`
  - `KYUUBIKI_DIRECT_MESH_TOKEN`
  - `KYUUBIKI_DIRECT_MESH_ALLOW_REQUEST_ENDPOINTS`
- watchdogs, heartbeat freshness, cancellation, and stale-job detection for long runs

Use these references as the current system handbook:

- [docs/system-overview.md](/Users/Shared/chroot/dev/kyuubiki/docs/system-overview.md)
- [docs/security.md](/Users/Shared/chroot/dev/kyuubiki/docs/security.md)
- [docs/operations.md](/Users/Shared/chroot/dev/kyuubiki/docs/operations.md)

The launcher reads this from [/.env.local](/Users/Shared/chroot/dev/kyuubiki/.env.local).

Convenience commands:

```bash
make start-local
make restart-local
make start-cloud
make restart-cloud
make start-distributed
make restart-distributed
```

`start-local` and `restart-local` force SQLite even if `.env.local` currently points at PostgreSQL. `start-cloud` and `restart-cloud` force PostgreSQL and require `DATABASE_URL`.

`start-distributed` and `restart-distributed` start only the control plane. They do not spawn local solver agents, so the orchestrator can dispatch into a remote cluster defined by either static endpoints or an agent manifest.

Remote solver registration API:

- `GET /api/v1/agents`
- `POST /api/v1/agents/register`
- `POST /api/v1/agents/:agent_id/heartbeat`
- `DELETE /api/v1/agents/:agent_id`

Main distributed deployment assets:

- [agents.local.json](/Users/Shared/chroot/dev/kyuubiki/deploy/agents.local.json)
- [agents.distributed.example.json](/Users/Shared/chroot/dev/kyuubiki/deploy/agents.distributed.example.json)
- [agent-manifest.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/agent-manifest.schema.json)

The Tauri installer GUI now also includes a `Remote` panel for:

- remote workspace bootstrap over `ssh`
- remote Rust agent launch over `ssh`
- distributed control-plane profile setup

## Project Format

Kyuubiki has a portable, engine-style project format built around JSON schemas.

Main schemas:

- [project.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/project.schema.json)
- [model.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/model.schema.json)
- [material-library.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/material-library.schema.json)

### Single-file project export

- extension: `.kyuubiki.json`
- schema: `kyuubiki.project/v2`

### Project archive export

- extension: `.kyuubiki`
- container: `zip`
- layout: `kyuubiki.project-layout/v1`

Current standardized archive layout:

```text
project.json
.kyuubiki/project.json
Assets/project/project.json
Assets/models/<model_id>.json
Assets/versions/<version_id>.json
ProjectSettings/workspace.json
ProjectSettings/automation-presets.json
ProjectSettings/asset-catalog.json
ProjectSettings/asset-references.json
Workspace/current-model.json
Analysis/jobs/index.json
Analysis/jobs/<job_id>.json
Analysis/results/index.json
Analysis/results/<job_id>.json
README.txt
```

The root `project.json` remains the canonical portable manifest. The hidden
`.kyuubiki/project.json` file describes the archive layout itself, while
`ProjectSettings/` holds workspace/editor state and automation presets. Detached
analysis payloads live under `Analysis/` so exported bundles can be reviewed
offline. `ProjectSettings/asset-catalog.json` adds stable asset-level GUID
tracking, `ProjectSettings/asset-references.json` captures guid-to-guid
relations, and core resources emit adjacent `*.meta` sidecars inside the
archive.
Legacy layout aliases are still emitted for compatibility with older imports.

## Installer

### Rust installer CLI

Kyuubiki includes a repo-local cross-platform installer utility built in Rust.

It currently supports:

- `doctor`
- `validate-env`
- `init-env`
- `prepare-layout`
- `export-launch`
- `stage-release`
- `bootstrap`

Use:

```bash
make doctor
make validate-env
make install ARGS="bootstrap"
make package
```

Installer source:

- [installer lib.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/installer/src/lib.rs)
- [installer main.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/installer/src/main.rs)

Current release scaffold:

```text
dist/<platform>/
  bin/
  config/
  data/
  logs/
  exports/
  manifests/
  scripts/
```

### Tauri installer GUI

Kyuubiki also includes a Tauri-based installer shell for a more visual setup flow.

- app root: [apps/installer-gui](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui)
- Tauri backend: [src-tauri/src/main.rs](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/src-tauri/src/main.rs)
- GUI shell: [ui/index.html](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui/index.html)

Common commands:

```bash
make installer-gui-dev
make installer-gui-build
```

### Tauri Hub GUI

Kyuubiki now also includes an emerging Tauri-based desktop hub that is meant to
be the daily launcher, runtime controller, and operator shell above installer
and workbench.

- app root: [apps/hub-gui](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui)
- Tauri backend: [src-tauri/src/main.rs](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/src-tauri/src/main.rs)
- hub shell UI: [ui/index.html](/Users/Shared/chroot/dev/kyuubiki/apps/hub-gui/ui/index.html)
- architecture notes:
  [docs/hub-architecture.md](/Users/Shared/chroot/dev/kyuubiki/docs/hub-architecture.md)

Common commands:

```bash
make hub-gui-dev
make hub-gui-build
```

### Tauri workbench GUI

Kyuubiki also includes a thin desktop shell for the workbench itself.

- app root: [apps/workbench-gui](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui)
- Tauri backend: [src-tauri/src/main.rs](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui/src-tauri/src/main.rs)
- desktop shell UI: [ui/index.html](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui/ui/index.html)

Common commands:

```bash
make workbench-gui-dev
make workbench-gui-build
```

The desktop shell currently provides:

- native service start/restart/stop controls
- local runtime status output
- embedded workbench view
- quick log inspection for `frontend`, `orchestrator`, and local agents

The GUI currently wraps:

- local/cloud mode selection
- environment validation
- bootstrap flow
- service control
- log viewing
- release scaffold generation

Shared installer/workbench branding now comes from:

- [assets/brand/brand.json](/Users/Shared/chroot/dev/kyuubiki/assets/brand/brand.json)

The installer consumes that branding through:

- [apps/installer-gui/ui/assets/brand.json](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui/assets/brand.json)
- [apps/installer-gui/ui/index.html](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui/index.html)
- [apps/installer-gui/ui/app.js](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui/app.js)

The frontend consumes the same branding through:

- [apps/frontend/src/app/layout.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/app/layout.tsx)
- [apps/frontend/src/components/workbench/workbench.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/components/workbench/workbench.tsx)

## Benchmarks

Benchmark runner:

- [benchmark main.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/benchmark/src/main.rs)

Profiles:

```bash
cd <repo>/workers/rust
cargo run -p kyuubiki-benchmark -- --profile medium --repeat 3
cargo run -p kyuubiki-benchmark -- --profile large --repeat 1
cargo run -p kyuubiki-benchmark -- --profile v2 --repeat 1
cargo run -p kyuubiki-benchmark -- --profile 10k --repeat 1
cargo run -p kyuubiki-benchmark -- --profile 15k --repeat 1
cargo run -p kyuubiki-benchmark -- --profile 20k --repeat 1
cargo run -p kyuubiki-benchmark -- --profile 10k --repeat 3 --baseline-out benchmarks/10k-baseline.json
cargo run -p kyuubiki-benchmark -- --profile 10k --repeat 1 --baseline-compare benchmarks/10k-baseline.json
cargo run -p kyuubiki-benchmark -- --profile 10k --repeat 1 --baseline-compare benchmarks/10k-baseline.json --compare-report-out benchmarks/reports/10k-compare.md
```

Or through Make:

```bash
make benchmark-baseline PROFILE=medium REPEAT=5
make benchmark-compare PROFILE=medium REPEAT=3
make benchmark-report PROFILE=10k REPEAT=1
make verify
```

Current benchmark tiers are aimed at:

- `medium`: small-to-mid validation runs
- `large`: low-thousands node class
- `v2`: path toward the next scale target
- `10k`: explicit single-machine `10k`-node route across `1D bar`, `2D truss`, `3D truss`, and `2D plane triangle`
- `15k`: upper single-machine exploration tier
- `20k`: stretch tier for machine-boundary probing

Benchmark table output now includes `median`, `p95`, and peak RSS so single-machine scale work can be tracked by both latency stability and memory pressure. Baseline snapshots can be written with `--baseline-out`, later runs can be compared against them with `--baseline-compare`, and human-readable Markdown reports can be emitted with `--compare-report-out`.

Checked-in single-machine baselines currently exist for:

- [medium-baseline.json](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/medium-baseline.json)
- [10k-baseline.json](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/10k-baseline.json)
- [15k-baseline.json](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/15k-baseline.json)
- [20k-baseline.json](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/20k-baseline.json)

Comparison reports are written under:

- [workers/rust/benchmarks/reports](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/reports)

`make verify` and CI use a lightweight regression gate against the checked-in `medium` baseline. Default guardrails are:

- repeat count: `3`
- median regression threshold: `25%`
- peak RSS regression threshold: `20%`
- ignore cases whose baseline median is under `1.0 ms`

## Local Development

Start everything:

```bash
cd <repo>
make start
```

Start in forced local SQLite mode:

```bash
make start-local
```

Check services:

```bash
make status
```

Restart services:

```bash
make restart
```

Export the current database snapshot:

```bash
make export-db > kyuubiki-database.json
```

Main endpoints:

- workbench: [http://127.0.0.1:3000](http://127.0.0.1:3000)
- orchestrator: [http://127.0.0.1:4000](http://127.0.0.1:4000)
- solver agents: `tcp://127.0.0.1:5001`, `tcp://127.0.0.1:5002`
