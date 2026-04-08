# kyuubiki v0.3

Kyuubiki is an engine-first FEM workstation and control plane with a browser-first workbench:

- `Next.js` workbench UI for modeling, project management, result review, and immersive 3D editing
- `Elixir` orchestrator API for jobs, persistence, chunked result delivery, and multi-agent coordination
- `Rust` solver agents for FEM data-plane computation, benchmarking, and engine-style reusable core logic

`v0.3` is the first version where Kyuubiki starts to feel like an engine-backed FEM product rather than a polished prototype. It now couples an editor-style workbench with chunked large-result handling, distributed agent orchestration, dual-database storage, installer flows, and benchmarked single-machine scaling through the `10k` to `20k` node class.

It also now has an explicit deployment split:

- `local workstation`: frontend + orchestrator + local Rust agents
- `cloud control plane`: frontend + orchestrator + PostgreSQL
- `distributed control plane`: orchestrator/frontend separated from remotely deployed solver nodes
- `headless peer mesh`: Rust solver agents can run without a GUI or Phoenix on the same host, and can advertise LAN peer-cluster topology through the shared RPC descriptor

And the frontend direction is now explicitly split too:

- `orchestrated_gui`
  the current workbench mode, where the editor talks to Phoenix and Phoenix
  talks to solver agents
- `direct_mesh_gui`
  a future mode where the editor can operate directly against a LAN peer mesh
  of headless Rust agents without requiring Phoenix on the solver hot path

And it now has an explicit protocol split:

- `kyuubiki.control-plane/http-v1`
- `kyuubiki.solver-rpc/v1`

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
- [docs/protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md)
- [scripts/README.md](/Users/Shared/chroot/dev/kyuubiki/scripts/README.md)

## What v0.3 Can Do

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
- editor-style compact UI with denser tabs, virtualized lists, and docked panels
- lazy-rendered large-result windows with chunk-aware browsing for `10k+` scale review

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
- desktop icon assets wired from `assets/icons`
- frontend browser/app icons wired from `assets/icons/app`

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

The next layer of decoupling is now explicit too:

- the frontend speaks the control-plane HTTP protocol
- the orchestrator speaks the solver RPC protocol
- solver agents can self-describe over RPC and the orchestrator can expose those descriptors over HTTP
- the frontend is being prepared to support both orchestrated and direct-mesh runtime modes as separate programs sharing common contracts

## Current Capabilities by Layer

### Frontend workbench

Main entry points:

- [page.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/app/page.tsx)
- [workbench.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/components/workbench/workbench.tsx)
- [workbench-viewport.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/components/workbench/workbench-viewport.tsx)

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
- a frontend runtime switch between:
  - `orchestrated_gui`
  - `direct_mesh_gui`
- direct-mesh frontend routes for LAN solver access:
  - `/api/direct-mesh/agents`
  - `/api/direct-mesh/solve`

### Orchestrator API

Main router:

- [router.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/router.ex)

Core orchestration:

- [analysis.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/analysis.ex)
- [library.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/library.ex)

The orchestrator currently provides:

- health endpoint
- protocol descriptor endpoints
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
- watchdog timeouts, stale-job detection, heartbeat surfacing, and cancel support for long-running jobs

### Rust engine and agents

Main crates:

- [protocol](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/protocol/src/lib.rs)
- [engine](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/engine/src/lib.rs)
- [solver](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/solver/src/lib.rs)
- [cli](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/cli/src/main.rs)
- [benchmark](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/benchmark/src/main.rs)

The Rust side currently provides:

- framed TCP RPC transport
- agent self-description via `describe_agent`
- generic runtime `ping` and `cancel_job` methods
- headless standalone agent mode
- peer-mesh runtime metadata (`runtime_mode`, `cluster_id`, `peers`)
- gossip-lite peer discovery between solver agents via `describe_agent`
- progress events
- multi-agent execution
- benchmark profiles
- dedicated `10k`, `15k`, and `20k` benchmark profiles for single-machine scale targets
- mixed dense/sparse and specialized solver paths
- result chunk helpers used by the engine/orchestrator split
- checked-in performance baselines and compare reports with regression gates

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
cd /Users/Shared/chroot/dev/kyuubiki/workers/rust
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

## v0.3 Direction

`v0.3` is about making Kyuubiki feel cohesive under load, not just feature-complete:

- engine-oriented separation between UI, orchestration, and solving
- stronger editor-style frontend ergonomics for 2D and 3D work
- viewport-aware large-result handling with chunking and progressive rendering
- stronger distributed-control-plane behavior with remote agents and runtime health
- benchmarked single-machine scaling with checked-in baselines and regression gates

The next scale target after `v0.3` is clear: keep `10k` as the comfort zone, treat `15k` as a stable upper tier, and use `20k` as the single-machine stretch band on an `M2 + 16GB` class machine while continuing to shift the browser toward viewport-driven data loading.

Recent single-machine scaling probes on the current Rust solver stack show:

- `15k` `2D truss`: `14884` nodes, `29768` DOF, about `9.19 s`, `61 MiB` peak RSS
- `20k` `2D truss`: `19881` nodes, `39762` DOF, about `14.72 s`, `78 MiB` peak RSS
- `15k` `2D plane triangle`: `14884` nodes, `29768` DOF, about `9.73 s`, `74 MiB` peak RSS
- `20k` `2D plane triangle`: `19881` nodes, `39762` DOF, about `15.36 s`, `78 MiB` peak RSS
- `15k` `3D truss`: `15138` nodes, `45414` DOF, about `228 ms`, `53 MiB` peak RSS
- `20k` `3D truss`: `20000` nodes, `60000` DOF, about `301 ms`, `60 MiB` peak RSS

Checked-in repeat-based baselines now also exist for:

- [15k-baseline.json](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/15k-baseline.json)
- [20k-baseline.json](/Users/Shared/chroot/dev/kyuubiki/workers/rust/benchmarks/20k-baseline.json)
