# Changelog

## v0.2

Kyuubiki `v0.2` is the first release where the system behaves like a coherent local-first FEM workbench rather than a loose prototype.

### Added

- Next.js workbench with:
  - `1D axial bar`
  - `2D truss`
  - `2D plane triangle`
  - `3D space truss`
- immersive `3D` workspace mode
- direct `2D` and `3D` node drag editing
- `3D` box selection, focus, frame selection, link editing, duplication, mirror, and nudge tools
- multi-material model support for `2D truss`, `3D truss`, and `2D plane triangle`
- external material import from `JSON` and `CSV`
- project / model / model-version CRUD
- job / result CRUD
- portable project formats:
  - `.kyuubiki.json`
  - `.kyuubiki`
- chunked result browsing for large result sets
- Rust engine facade crate
- benchmark profiles: `medium`, `large`, `v2`
- Rust installer CLI
- Tauri installer GUI

### Changed

- moved toward engine-first separation between frontend, orchestrator, and solver
- added multi-agent Rust RPC execution with round-robin dispatch and failover
- added dual database support:
  - local-first `SQLite`
  - distributed/cloud `PostgreSQL`
- improved 3D workspace layout so the viewport can fully occupy space when auxiliary docks are closed
- reworked frontend into a denser, more ergonomic workbench with tabbed panels and virtualized lists

### Persistence

- persisted projects
- persisted models
- persisted model versions
- persisted jobs
- persisted results
- database snapshot export

### Tooling

- `make start-local` / `make restart-local`
- `make start-cloud` / `make restart-cloud`
- `make doctor`
- `make validate-env`
- `make export-db`
- `make installer-gui-dev`
- `make installer-gui-build`

### Direction after v0.2

- single-machine `10k`-node workflows on `M2 + 16GB`
- stronger sparse-first solver paths
- more engine-style result chunking and viewport-driven loading
