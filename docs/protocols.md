# Protocols

Kyuubiki `v0.3` treats the GUI, control plane, and solver agents as three
independent programs.

## Program split

- `kyuubiki-frontend`
  Browser-first GUI and editor shell. It should only consume stable HTTP APIs
  and result windows in orchestrated mode, and should be able to speak to
  headless Rust agents directly in direct-mesh mode.
- `kyuubiki-orchestrator`
  Control plane for jobs, persistence, scheduling, health, and distributed
  agent coordination.
- `kyuubiki-rust-agent`
  Compute-plane program that exposes solver capabilities over framed TCP RPC.
  It is intentionally headless and can run:
  - standalone
  - orchestrated by Phoenix
  - as part of a peer mesh on a LAN

## Public protocols

### Frontend runtime modes

The frontend is expected to split into two independent operating modes:

- `orchestrated_gui`
  - talks to `kyuubiki.control-plane/http-v1`
  - best for centralized persistence, cloud control planes, and distributed
    solver fleets
- `direct_mesh_gui`
  - talks directly to headless `kyuubiki.solver-rpc/v1` agents on a LAN
  - best for local peer meshes where the control plane is optional rather than
    mandatory

Both modes should share the same editor shell and model/result contracts, while
remaining independent at runtime.

### Control plane

- Name: `kyuubiki.control-plane/http-v1`
- Transport: `HTTP + JSON`
- Primary descriptor endpoint: `/api/v1/protocol`

The control plane exposes:

- health: `/api/health`
- protocol descriptor: `/api/v1/protocol`
- control-plane protocol descriptor: `/api/v1/protocol/control-plane`
- solver RPC compatibility descriptor: `/api/v1/protocol/solver-rpc`
- reachable agent descriptors: `/api/v1/protocol/agents`
- job/result/project/model CRUD over `/api/v1/...`

### Solver RPC

- Name: `kyuubiki.solver-rpc/v1`
- RPC version: `1`
- Transport: `TCP`
- Framing: `length_prefixed_u32`
- Encoding: `JSON`

The current generic/runtime methods are:

- `ping`
- `describe_agent`
- `cancel_job`

The current solver methods are:

- `solve_bar_1d`
- `solve_truss_2d`
- `solve_truss_3d`
- `solve_plane_triangle_2d`

Progress and runtime liveness are streamed back as RPC frames:

- `progress`
- `heartbeat`

Solver agents also self-describe their runtime topology:

- `runtime_mode`
  - `standalone`
  - `orchestrated`
  - `peer_mesh`
- `cluster_id`
- `headless`
- `peers[]`

In peer-mesh mode the current behavior is intentionally lightweight:

- each agent starts from configured seed peers
- it periodically calls `describe_agent` on those peers
- it merges the returned peer lists into its own runtime view

This gives Kyuubiki a gossip-lite LAN discovery layer without introducing a
hard dependency on Phoenix for solver-to-solver awareness.

## Why this matters

This split keeps deployment modes independent:

- local workstation
- cloud control plane
- distributed control plane with remote solver nodes
- headless peer-mesh solver clusters on a LAN
- future direct-mesh frontends that bypass Phoenix on the solver hot path

As long as the programs speak these public contracts, the frontend, Phoenix
control plane, and Rust agents can evolve on different release cadences without
re-coupling at the implementation level.
