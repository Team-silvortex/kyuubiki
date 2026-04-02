# kyuubiki

Kyuubiki is a browser-first FEM workbench with a split architecture:

- `Next.js` workbench UI for modeling, project management, and results review
- `Elixir` orchestrator API for jobs, persistence, and coordination
- `Rust` solver agents for the actual FEM data-plane computation

The current stack is already runnable on macOS and now supports both `SQLite` and `PostgreSQL` for persisted jobs, projects, models, and model versions.

## Cross-platform Bootstrap

Kyuubiki now includes a repo-local cross-platform installer utility built in Rust.

It does not try to silently mutate host-level package managers. Instead it helps with:

- environment checks
- `.env.local` variable validation
- repo-local directory preparation
- `.env.local` initialization
- launch manifest export for future packaging/deployment work
- portable release directory scaffolding under `dist/`

Use:

```bash
make doctor
make validate-env
make install ARGS="bootstrap"
make package
```

Installer source:

- [installer main.rs](/Users/Shared/chroot/dev/kyuubiki/workers/rust/crates/installer/src/main.rs)

Current release scaffold output:

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
- Job / result CRUD
- Database snapshot export
- Undo / redo for frontend modeling actions
- Persistent job history
- Benchmark runner for solver cases
- Project bundle export and import

## Project Storage

Kyuubiki now supports two SQL storage modes through the Elixir orchestrator:

- `sqlite`
  Best default for local single-machine work. Minimal host burden and zero extra service management.
- `postgres`
  Recommended for cloud, multi-node, and distributed deployments.

Persisted entities include:

- jobs
- analysis results
- projects
- models
- model versions

Recommended local setup:

```bash
KYUUBIKI_STORAGE_BACKEND=sqlite
SQLITE_DATABASE_PATH=/Users/Shared/chroot/dev/kyuubiki/tmp/data/kyuubiki_dev.sqlite3
```

Recommended cloud/distributed setup:

```bash
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
```

`start-local` and `restart-local` force SQLite even if `.env.local` currently points at PostgreSQL. `start-cloud` and `restart-cloud` force PostgreSQL and require `DATABASE_URL`.

## Installer GUI

Kyuubiki now includes a Tauri-based installer shell for a more visual cross-platform setup flow.

- GUI app root: [apps/installer-gui](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui)
- Tauri backend: [src-tauri/src/main.rs](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/src-tauri/src/main.rs)
- Static installer UI: [ui/index.html](/Users/Shared/chroot/dev/kyuubiki/apps/installer-gui/ui/index.html)

Common commands:

```bash
make installer-gui-dev
make installer-gui-build
```

The GUI wraps the same Rust installer logic used by `kyuubiki-installer`, including doctor, env validation, layout prep, bootstrap, and release staging.

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
- database snapshot export as JSON
- system-side job/result record management
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
- job CRUD
- result CRUD
- full database export
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

Benchmark profiles:

```bash
cd /Users/Shared/chroot/dev/kyuubiki/workers/rust
cargo run -p kyuubiki-benchmark -- --profile medium --repeat 3
cargo run -p kyuubiki-benchmark -- --profile large --repeat 1
cargo run -p kyuubiki-benchmark -- --profile v2 --repeat 1
```

Current benchmark tiers are aimed at:

- `medium`: small-to-mid validation runs
- `large`: around the low-thousands node class
- `v2`: around the 5000-node class target

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

Export the current database snapshot:

```bash
make export-db > kyuubiki-database.json
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
