# kyuubiki

Kyuubiki is a browser-first FEM workbench with a split architecture:

- `Next.js` workbench UI for modeling, project management, and results review
- `Elixir` orchestrator API for jobs, persistence, and coordination
- `Rust` solver agents for the actual FEM data-plane computation

The current stack is already runnable on macOS and uses `PostgreSQL` for persisted jobs, projects, models, and model versions.

## Current Architecture

```text
Next.js workbench (3000)
  -> Elixir orchestrator API (4000)
    -> Rust TCP solver agent pool (5001, 5002, ...)
      -> FEM kernels (1D / 2D / 3D)

PostgreSQL
  <- jobs / results / projects / models / model_versions
```

## Current Features

- 1D axial bar studies
- 2D truss studies
- 2D plane triangle studies
- 3D space truss studies
- Parametric model generation
- Direct node drag editing for 2D truss
- Project / model / version CRUD
- Undo / redo for frontend modeling actions
- Persistent job history
- Benchmark runner for solver cases
- Project bundle export and import

## Project Storage

Local persistence now uses PostgreSQL through the Elixir orchestrator.

Persisted entities include:

- jobs
- analysis results
- projects
- models
- model versions

On this machine the default local setup is:

```bash
KYUUBIKI_STORAGE_BACKEND=postgres
DATABASE_URL=ecto://seis@127.0.0.1:5432/kyuubiki_dev
```

The launcher reads this from [/.env.local](/Users/Shared/chroot/dev/kyuubiki/.env.local).

## Project Format

Kyuubiki now has a dedicated JSON-based portable project format.

### 1. Single-file project export

- extension: `.kyuubiki.json`
- schema: `kyuubiki.project/v1`

Main schema:

- [project.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/project.schema.json)

Related model schema:

- [model.schema.json](/Users/Shared/chroot/dev/kyuubiki/schemas/model.schema.json)

### 2. Project archive export

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

The root `project.json` remains the canonical manifest so imports stay simple and backward-compatible.
Detached analysis payloads now live under `results/` inside the zip archive so exported bundles can be reviewed offline.

## Browser Workbench

The Next.js workbench currently supports:

- sample library browsing
- project CRUD
- saved model CRUD
- model version management
- result export as JSON / CSV
- project export as `.kyuubiki.json` and `.kyuubiki`
- project import from both formats

Main frontend entry points:

- [page.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/app/page.tsx)
- [workbench.tsx](/Users/Shared/chroot/dev/kyuubiki/apps/frontend/src/components/workbench.tsx)

## Orchestrator API

The Elixir app currently serves:

- health
- FEM job submission
- job lookup
- project CRUD
- model CRUD
- model version CRUD
- round-robin dispatch across multiple Rust RPC agents with failover for unavailable endpoints

Main API router:

- [router.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/router.ex)

Persistence and library facade:

- [library.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/library.ex)
- [schema_setup.ex](/Users/Shared/chroot/dev/kyuubiki/apps/web/lib/kyuubiki_web/storage/schema_setup.ex)

## Rust Solver Agent

The Rust side currently provides:

- framed TCP RPC transport
- progress events
- 1D axial bar solve
- 2D truss solve
- 2D plane triangle solve
- 3D truss solve
- benchmark executable

The orchestrator can target multiple local or remote agents through:

```bash
KYUUBIKI_AGENT_ENDPOINTS=127.0.0.1:5001,127.0.0.1:5002
```

Main Rust crates:

- [protocol](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/protocol/src/lib.rs)
- [solver](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/solver/src/lib.rs)
- [cli](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/cli/src/main.rs)
- [benchmark](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/benchmark/src/main.rs)

## Local Development

Start everything:

```bash
cd /Users/Shared/chroot/dev/kyuubiki
make start
```

Check services:

```bash
make status
```

Restart services:

```bash
make restart
```

Main local endpoints:

- workbench: [http://127.0.0.1:3000](http://127.0.0.1:3000)
- orchestrator: [http://127.0.0.1:4000](http://127.0.0.1:4000)
- solver agents: `tcp://127.0.0.1:5001`, `tcp://127.0.0.1:5002`

## Verification

Run the full verification suite:

```bash
make verify
```

This currently covers:

- Elixir tests
- Rust tests
- browser FEM tests
- formatting checks

## Near-term Direction

The next natural steps are:

- connect jobs to `model_version_id`
- add richer 3D modeling controls
- move from polling to streaming job progress
