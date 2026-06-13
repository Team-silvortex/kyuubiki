# System Overview

Kyuubiki is now best understood as three cooperating programs with a shared
contract surface:

- `Frontend GUI`
  The browser workbench and installer GUI.
- `Control plane`
  The Phoenix/Plug orchestration layer.
- `Solver data plane`
  Rust engine crates and headless Rust agents.

In the `tamamono 1.x` line, those programs now carry a broader operator
family than the early truss-only baseline. The shared stack spans:

- axial and thermal bars
- spring studies in `1D / 2D / 3D`
- beams, torsion shafts, and `2D` frames
- truss studies in `2D / 3D`
- plane studies with triangle and quad elements

On the desktop product side, Kyuubiki also now needs three cooperating GUI
surfaces:

- `Hub`
  desktop entrypoint, runtime launcher, and operator shell
- `Workbench`
  focused modeling and analysis surface
- `Installer`
  bootstrap and heavier deployment setup surface

Those GUI surfaces and runtime layers are intentionally not the same thing.

- `Hub` is the system entrypoint, not the control plane
- `Workbench` is the engineering workflow surface, not the runtime fleet manager
- `Installer` is the deployment/lifecycle surface, not the modeling surface
- `agent` and `runtime` layers are execution peers, not frontend submodules

## Repository anchor

Those runtime layers map cleanly onto the monorepo:

- `apps/frontend`
  browser workbench and UI state
- `apps/web`
  control-plane API and persistence
- `workers/rust`
  solver kernels, engine crates, agents, and CLI runtime

See [repository-structure.md](repository-structure.md)
for the fuller directory map.

The relationship between `Hub` and `control plane` is intentional:

- the Hub is not the control plane
- the control plane is one runtime workload the Hub can manage
- a local orchestrator is only the default managed target, not the only one

That design keeps the desktop entrypoint compatible with:

- local single-machine stacks
- remote control planes
- several control-plane targets in one operator session
- local bundle and packaging work even when no control plane is running

See [app-runtime-boundaries.md](app-runtime-boundaries.md)
for the stricter role split.

## Product boundary

At the product level, each major surface owns a different concern:

- `Hub`
  global workload shell, runtime target overview, launch surface
- `Workbench`
  concrete modeling, workflow, and result interaction surface
- `Installer`
  deployment, installation, integrity, update, and cleanup surface
- `Runtime / agents`
  protocol-driven execution and compute surface

That boundary matters because it prevents two common failures:

- frontend code quietly becoming the runtime architecture
- runtime code quietly absorbing UI-specific assumptions

## Runtime split

### `orchestrated_gui`

The current default product mode:

- browser workbench
- Phoenix/Plug orchestrator
- one or more Rust agents
- SQLite or PostgreSQL persistence

This mode is the best fit for:

- local workstation use
- central cluster control
- persistent projects, jobs, and results
- watchdog and administrative workflows

### `direct_mesh_gui`

A lighter path where the GUI talks directly to LAN agents:

- browser workbench
- Next.js direct-mesh routes
- one or more headless Rust agents

This mode is the best fit for:

- LAN solver clusters
- lower-latency direct solve experimentation
- environments where Phoenix should stay out of the hot path

### `headless peer mesh`

Rust agents can now run without a GUI or orchestrator:

- standalone
- orchestrated
- peer mesh

Peer mesh mode currently covers:

- self-description
- cluster identity
- lightweight peer gossip
- health scoring

## Responsibilities by layer

### Frontend GUI

- modeling
- viewport interaction
- material editing
- project and result browsing
- chunked large-result review
- direct mesh runtime selection

The frontend may observe and command runtimes, but it should not define runtime
shape or agent architecture.

### Control plane

- job submission
- persistence
- cluster-aware routing
- watchdog scanning
- cancellation
- result chunk APIs
- remote agent registration

When viewed from the Hub, the control plane behaves as a managed runtime target
rather than as the desktop shell itself.

### Solver data plane

- FEM kernels
- benchmark and baseline tooling
- framed solver RPC
- progress and heartbeat frames
- cluster self-description

The solver/runtime layer should remain usable through Hub, Workbench,
Installer-driven flows, or headless SDK clients without inheriting frontend
implementation details.

## Shared contracts

The layers intentionally meet at protocol edges rather than shared runtime
implementation:

- `kyuubiki.control-plane/http-v1`
- `kyuubiki.solver-rpc/v1`
- versioned JSON schemas in `schemas/`

See:

- [protocols.md](protocols.md)
- [security.md](security.md)
- [operations.md](operations.md)
