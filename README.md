# kyuubiki v0.2

Kyuubiki is a browser-first FEM workbench with an engine-first split architecture:

- `Next.js` workbench UI for modeling, project management, result review, and immersive 3D editing
- `Elixir` orchestrator API for jobs, persistence, chunked result delivery, and multi-agent coordination
- `Rust` solver agents for FEM data-plane computation, benchmarking, and engine-style reusable core logic

`v0.2` is the first version where the stack feels like a real product rather than a prototype. It now supports persistent projects, chunked large-result browsing, immersive 3D editing, dual-database storage, portable project bundles, and a cross-platform installer GUI.

It also now has an explicit deployment split:

- `local workstation`: frontend + orchestrator + local Rust agents
- `cloud control plane`: frontend + orchestrator + PostgreSQL
- `distributed control plane`: orchestrator/frontend separated from remotely deployed solver nodes

## Repository Shape

The monorepo is intentionally split by responsibility:

- `apps/`
  Product-facing surfaces such as the browser workbench, orchestrator API, and
  installer GUI
- `workers/`
  Rust data-plane crates, solver runtime, benchmark tooling, and installer CLI
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
- [docs/repository-structure.md](/Users/Shared/chroot/dev/kyuubiki/docs/repository-structure.md)

## What v0.2 Can Do

### Solvers

- `1D axial bar`
- `2D truss`
- `2D plane triangle`
- `3D space truss`

### Modeling and workbench

- parameterized study setup
- direct node drag editing in `2D truss`
- direct node drag editing in `3D truss`
- link / unlink member editing
- node duplication and axis mirror tools in `3D`
- box selection, multi-selection, focus, frame selection, and nudge tools in `3D`
- immersive 3D workspace mode
- undo / redo for frontend modeling actions
- compact tabbed UI with virtualized lists and chunk-aware large-result browsing

### Materials

- multi-material support inside a single `2D truss`, `3D truss`, or `2D plane triangle` model
- material library in the workbench
- apply material to selected entities
- apply material to whole model
- material color visualization in the viewport
- material legend and hide/show filtering
- custom material authoring
- material import from external `JSON` and `CSV`

### Projects and persistence

- project CRUD
- model CRUD
- model version CRUD
- job CRUD
- result CRUD
- database snapshot export
- persistent job history
- project bundle import/export

### Installer and packaging

- Rust installer CLI
- Tauri installer GUI
- environment validation
- local SQLite mode
- cloud PostgreSQL mode
- release scaffold generation under `dist/`

## Current Architecture

```text
Next.js workbench (3000)
  -> Elixir orchestrator API (4000)
    -> Rust TCP solver agent pool (5001, 5002, ...)
      -> Rust engine + solver kernels

SQLite or PostgreSQL
  <- projects / models / model_versions / jobs / results
```

Distributed deployments can now discover agents in two independent ways:

- `static`: `KYUUBIKI_AGENT_ENDPOINTS=host:port,...`
- `manifest`: [agent-manifest.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/agent-manifest.schema.json) via `KYUUBIKI_AGENT_MANIFEST_PATH`
- `registry`: remote Rust agents register and heartbeat into the orchestrator over HTTP

The direction is engine-first:

- Rust `engine` owns reusable solving and result-window helpers
- Phoenix/Plug orchestration owns jobs, storage, and HTTP APIs
- frontend consumes stable APIs instead of solver internals

## Current Capabilities by Layer

### Frontend workbench

Main entry points:

- [page.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/app/page.tsx)
- [workbench.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/components/workbench.tsx)
- [workbench-viewport.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/components/workbench-viewport.tsx)

The workbench currently supports:

- sample library browsing
- project and model library management
- model versioning
- result export as `JSON` and `CSV`
- database snapshot export as `JSON`
- `.kyuubiki.json` export/import
- `.kyuubiki` archive export/import
- chunked browsing of large result sets
- immersive 3D workspace with externalized tools/help panels
- local settings persistence for theme, language, and shortcut hints

### Orchestrator API

Main router:

- [router.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/router.ex)

Core orchestration:

- [analysis.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/analysis.ex)
- [library.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/library.ex)

The orchestrator currently provides:

- health endpoint
- FEM job submission
- job lookup and job history
- project/model/model-version CRUD
- result CRUD
- full database export
- chunked result APIs for large result windows
- round-robin dispatch across multiple Rust RPC agents
- failover when an agent endpoint is unavailable
- deployment-aware health reporting
- manifest-based remote agent discovery for distributed control-plane setups
- runtime agent registration, heartbeat, and removal APIs for remote solver nodes

### Rust engine and agents

Main crates:

- [protocol](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/protocol/src/lib.rs)
- [engine](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/engine/src/lib.rs)
- [solver](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/solver/src/lib.rs)
- [cli](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/cli/src/main.rs)
- [benchmark](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/benchmark/src/main.rs)

The Rust side currently provides:

- framed TCP RPC transport
- progress events
- multi-agent execution
- benchmark profiles
- dedicated `10k` benchmark profile for single-machine scale targets
- mixed dense/sparse and specialized solver paths
- result chunk helpers used by the engine/orchestrator split

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
SQLITE_DATABASE_PATH=/Users/Shared/chroot/dev/kyuubiki/tmp/data/kyuubiki_dev.sqlite3
KYUUBIKI_AGENT_ENDPOINTS=127.0.0.1:5001,127.0.0.1:5002
```

Recommended cloud/distributed setup:

```bash
KYUUBIKI_DEPLOYMENT_MODE=distributed
KYUUBIKI_AGENT_DISCOVERY=registry
KYUUBIKI_AGENT_MANIFEST_PATH=/Users/Shared/chroot/dev/kyuubiki/deploy/agents.distributed.example.json
KYUUBIKI_STORAGE_BACKEND=postgres
DATABASE_URL=ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev
```

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

Kyuubiki has a portable project format built around JSON schemas.

Main schemas:

- [project.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/project.schema.json)
- [model.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/model.schema.json)
- [material-library.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/material-library.schema.json)

### Single-file project export

- extension: `.kyuubiki.json`
- schema: `kyuubiki.project/v1`

### Project archive export

- extension: `.kyuubiki`
- container: `zip`

Current archive layout:

```text
project.json
project/project.json
models/<model_id>.json
versions/<version_id>.json
jobs/jobs.json
jobs/<job_id>.json
results/results.json
results/<job_id>.json
workspace/manifest.json
workspace/current-model.json
README.txt
```

The root `project.json` remains the canonical manifest. Detached analysis payloads live under `results/` so exported bundles can be reviewed offline.

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

The GUI currently wraps:

- local/cloud mode selection
- environment validation
- bootstrap flow
- service control
- log viewing
- release scaffold generation

## Benchmarks

Benchmark runner:

- [benchmark main.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/benchmark/src/main.rs)

Profiles:

```bash
cd /Users/Shared/chroot/dev/kyuubiki/workers/rust
cargo run -p kyuubiki-benchmark -- --profile medium --repeat 3
cargo run -p kyuubiki-benchmark -- --profile large --repeat 1
cargo run -p kyuubiki-benchmark -- --profile v2 --repeat 1
cargo run -p kyuubiki-benchmark -- --profile 10k --repeat 1
```

Current benchmark tiers are aimed at:

- `medium`: small-to-mid validation runs
- `large`: low-thousands node class
- `v2`: path toward the next scale target
- `10k`: explicit single-machine `10k`-node route, starting with `2D truss` and `3D truss`

## Local Development

Start everything:

```bash
cd /Users/Shared/chroot/dev/kyuubiki
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

## Verification

Run the verification suite:

```bash
make verify
```

This currently covers:

- Elixir tests
- Rust tests
- frontend build/type checks
- browser FEM tests
- formatting checks

## v0.2 Direction

`v0.2` is about making Kyuubiki feel coherent end-to-end:

- engine-oriented separation between UI, orchestration, and solving
- better large-result handling through chunked APIs
- stronger immersive 3D editing
- stronger local-first setup through SQLite and installer UX

The next scale target after `v0.2` is clear: make single-machine `10k`-node workflows practical on an `M2 + 16GB` class machine, starting with `2D truss`.
