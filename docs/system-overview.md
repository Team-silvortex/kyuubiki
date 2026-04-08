# System Overview

Kyuubiki is now best understood as three cooperating programs with a shared
contract surface:

- `Frontend GUI`
  The browser workbench and installer GUI.
- `Control plane`
  The Phoenix/Plug orchestration layer.
- `Solver data plane`
  Rust engine crates and headless Rust agents.

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

### Control plane

- job submission
- persistence
- cluster-aware routing
- watchdog scanning
- cancellation
- result chunk APIs
- remote agent registration

### Solver data plane

- FEM kernels
- benchmark and baseline tooling
- framed solver RPC
- progress and heartbeat frames
- cluster self-description

## Shared contracts

The layers intentionally meet at protocol edges rather than shared runtime
implementation:

- `kyuubiki.control-plane/http-v1`
- `kyuubiki.solver-rpc/v1`
- versioned JSON schemas in `schemas/`

See:

- [protocols.md](/Users/Shared/chroot/dev/kyuubiki/docs/protocols.md)
- [security.md](/Users/Shared/chroot/dev/kyuubiki/docs/security.md)
- [operations.md](/Users/Shared/chroot/dev/kyuubiki/docs/operations.md)
