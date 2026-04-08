# Changelog

## v0.3

Kyuubiki `v0.3` is the release where the system starts to behave like an engine-backed FEM workstation under real scale, not just a coherent local-first prototype.

### Added

- formal benchmark scaling tiers for `10k`, `15k`, and `20k`
- checked-in single-machine baselines for `medium`, `10k`, `15k`, and `20k`
- benchmark comparison reports and regression gates
- progressive/lazy rendering for large viewport result windows
- adaptive chunk windows with jump navigation for large result browsing
- watchdog-backed job timeout, stale detection, heartbeat status, and cancel flows
- runtime remote-agent registration and heartbeat APIs for distributed deployments
- explicit control-plane and solver-RPC protocol descriptors
- Rust agent self-description and generic runtime RPC methods (`ping`, `describe_agent`)
- headless agent runtime metadata for standalone, orchestrated, and peer-mesh cluster modes
- gossip-lite peer discovery for LAN solver meshes
- explicit frontend runtime split in the architecture:
  - `orchestrated_gui`
  - `direct_mesh_gui`
- direct-mesh frontend API routes that let the Next.js shell inspect and solve
  against LAN Rust agents without going through Phoenix

### Changed

- pushed sparse-first solver performance further for `2D truss`, `2D plane triangle`, and `3D truss`
- improved single-machine `M2 + 16GB` behavior through `10k` and into the `15k`/`20k` node class
- tightened the frontend toward a denser editor-style layout with more segmented tabs and less card sprawl
- continued separating engine, orchestrator, installer, and workbench responsibilities
- made the GUI, control plane, and solver agents more explicitly deployable as independent programs
- clarified that the future frontend can run either through Phoenix or directly
  against a LAN peer mesh while sharing the same contracts

### Scale snapshot

- `10k` is now the practical comfort tier
- `15k` is a stable upper tier
- `20k` is a real single-machine stretch tier, with model-family-dependent cost

### Direction after v0.3

- push viewport-driven chunk loading beyond page-style result windows
- keep improving sparse solver stability and performance before chasing larger raw node counts
- deepen distributed orchestration and remote deployment workflows without coupling them to any single frontend mode

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
