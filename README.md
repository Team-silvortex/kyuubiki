# kyuubiki tamamono 1.4.0

Kyuubiki is an engine-first FEM workstation and control plane with a browser-first workbench:

- `Next.js` workbench UI for modeling, project management, result review, and immersive 3D editing
- `Elixir` orchestrator API for jobs, persistence, chunked result delivery, and multi-agent coordination
- `Rust` solver agents for FEM data-plane computation, benchmarking, and engine-style reusable core logic
- an emerging `Kyuubiki Hub` desktop shell for project launch, runtime control, and operator-facing orchestration

`tamamono 1.4.0` is the current repository-wide version line.

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

Start here if you need orientation:

- [docs/README.md](docs/README.md)
  Full current documentation map.
- [docs/current-line.md](docs/current-line.md)
  Short current-line posture for `tamamono 1.x`.
- [docs/system-overview.md](docs/system-overview.md)
  Runtime map across GUI, control plane, and solver data plane.

## Read By Intent

- new to the project:
  [docs/README.md](docs/README.md),
  [docs/system-overview.md](docs/system-overview.md),
  [docs/repository-structure.md](docs/repository-structure.md)
- changing protocols or automation surfaces:
  [docs/philosophy.md](docs/philosophy.md),
  [docs/protocols.md](docs/protocols.md),
  [docs/headless-sdks.md](docs/headless-sdks.md)
- changing tests, CI, or release flow:
  [docs/testing-and-ci.md](docs/testing-and-ci.md),
  [docs/packaging-and-deployment.md](docs/packaging-and-deployment.md),
  [docs/desktop-release-checklist.md](docs/desktop-release-checklist.md)
- changing workbench UX or visual language:
  [docs/philosophy.md](docs/philosophy.md),
  [docs/frontend-style.md](docs/frontend-style.md),
  [docs/frontend-implementation.md](docs/frontend-implementation.md),
  [apps/frontend/README.md](apps/frontend/README.md)

## What tamamono 1.4.0 Can Do

### Supported analysis domains

- `Mechanical`
  axial, spring, beam, torsion, truss, frame, and plane studies
- `Thermal`
  heat-bar and heat-plane studies with temperature and flux review
- `Thermo-mechanical`
  thermal bar, beam, frame, truss, and plane response paths

### Supported study families

The strongest currently verified families are:

- `axial_bar_1d`, `spring_1d`, `beam_1d`, `torsion_1d`
- `frame_2d`, `frame_3d`
- `truss_2d`, `truss_3d`
- `plane_triangle_2d`, `plane_quad_2d`
- `heat_bar_1d`, `heat_plane_triangle_2d`, `heat_plane_quad_2d`
- `thermal_bar_1d`, `thermal_beam_1d`, `thermal_frame_2d`, `thermal_frame_3d`
- `thermal_truss_2d`, `thermal_truss_3d`
- `thermal_plane_triangle_2d`, `thermal_plane_quad_2d`

Lighter but supported operator families:

- `spring_2d`
- `spring_3d`

### Product Surfaces

- `Workbench`
  modeling, materials, results, and analysis workflows
- `Hub`
  desktop entry shell, runtime control, bundle tools, and operator guidance
- `Installer`
  bootstrap, deployment, and packaging setup

### Current validation posture

- formal solver accuracy baselines for the current verified families
- orchestrated and direct-mesh smoke for supported study families
- Workbench sample open, report, and export coverage across the main domains
- repo-level validation through web, Rust, frontend, SDK, integration, and desktop baselines

For the current product-line posture, major-version policy, and quality
direction, use:

- [docs/current-line.md](docs/current-line.md)
- [docs/version-line.md](docs/version-line.md)
- [docs/tamamono-minor-lines.md](docs/tamamono-minor-lines.md)
- [docs/accuracy-plan.md](docs/accuracy-plan.md)
- [docs/accuracy-baselines.md](docs/accuracy-baselines.md)

Packaging and deployment paths are now documented centrally in:

- [docs/packaging-and-deployment.md](docs/packaging-and-deployment.md)
- [docs/desktop-release-checklist.md](docs/desktop-release-checklist.md)

Desktop release planning and platform bundle checks now live in the two docs
above rather than in the repo root.

## Verification

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

The packaging/output map and smoke breakdown live in:

- [docs/packaging-and-deployment.md](docs/packaging-and-deployment.md)
- [docs/testing-and-ci.md](docs/testing-and-ci.md)
- [tests/integration/README.md](tests/integration/README.md)

## Deployment Notes

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

Current security notes, project format details, installer flows, benchmark usage, and local development commands live in dedicated docs:

- [docs/security.md](docs/security.md)
- [docs/operations.md](docs/operations.md)
- [docs/packaging-and-deployment.md](docs/packaging-and-deployment.md)
- [docs/desktop-release-checklist.md](docs/desktop-release-checklist.md)
- [docs/repository-structure.md](docs/repository-structure.md)
