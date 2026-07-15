# System Overview

Kyuubiki is now best understood as three cooperating programs with a shared
contract surface:

- `Frontend GUI`
  The browser workbench and installer GUI.
- `Control plane`
  The Phoenix/Plug orchestration layer.
- `Solver data plane`
  Rust engine crates and headless Rust agents.

In the `moxi 2.x` line, those programs now carry a broader operator
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

For the stricter runtime-side split between Rust agents, Elixir orchestration,
and product shells, see
[agent-orchestrator-boundary.md](agent-orchestrator-boundary.md).

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

Use this file for the whole-system picture, not for the full product-role rule
book.

The short version is:

- `Hub` owns the desktop entry and workload shell
- `Workbench` owns engineering workflow interaction
- `Installer` owns deployment, integrity, update, and remote-node lifecycle
- `Runtime / agents` own protocol-driven execution

The deeper role split lives in
[app-runtime-boundaries.md](app-runtime-boundaries.md).
The Installer-owned remote node surface is described in
[installer-remote-control.md](installer-remote-control.md).

Those boundaries exist to prevent two common failures:

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

Rust agents can now run without a GUI or orchestrator in:

- standalone
- orchestrated
- peer mesh

For authority rules around peer mesh versus orchestrator-managed nodes, use:

- [agent-control-authority.md](agent-control-authority.md)
- [headless-agent-contract.md](headless-agent-contract.md)

## Responsibilities by layer

### Frontend GUI

Owns product interaction and runtime selection, but not runtime architecture.
See [app-runtime-boundaries.md](app-runtime-boundaries.md).

### Control plane

Owns job submission, persistence, routing, watchdog work, and remote-agent
coordination.

### Solver data plane

Owns FEM kernels, solver RPC, progress signaling, cluster self-description, and
compute-side benchmarking.

For the stricter runtime-side split between control plane, agents, frontend
gateways, and headless callers, use
[agent-orchestrator-boundary.md](agent-orchestrator-boundary.md).

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
- [installer-remote-control.md](installer-remote-control.md)
