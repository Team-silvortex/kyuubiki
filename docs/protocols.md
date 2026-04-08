# Protocols

Kyuubiki `v0.3` treats the GUI, control plane, and solver agents as three
independent programs.

## Program split

- `kyuubiki-frontend`
  Browser-first GUI and editor shell. It should only consume stable HTTP APIs
  and result windows.
- `kyuubiki-orchestrator`
  Control plane for jobs, persistence, scheduling, health, and distributed
  agent coordination.
- `kyuubiki-rust-agent`
  Compute-plane program that exposes solver capabilities over framed TCP RPC.

## Public protocols

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

## Why this matters

This split keeps deployment modes independent:

- local workstation
- cloud control plane
- distributed control plane with remote solver nodes

As long as the programs speak these public contracts, the frontend, Phoenix
control plane, and Rust agents can evolve on different release cadences without
re-coupling at the implementation level.
