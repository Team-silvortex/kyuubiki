# Headless SDKs

Kyuubiki now ships a dedicated `sdks/` top-level directory for protocol-first,
headless integrations.

## Why these SDKs exist

The browser workbench is becoming a powerful editor and operator shell, but AI
models and automation systems should not need to drive a GUI to use Kyuubiki.

The headless SDK layer gives them a cleaner tool surface:

- discover the running deployment
- inspect protocol compatibility
- submit FEM jobs
- poll job state
- describe reachable solver agents
- talk directly to solver RPC agents when the control plane is optional

## Language targets

- Rust
- Elixir
- Python

All three SDKs expose the same conceptual split:

- `ControlPlaneClient`
- `SolverRpcClient`

## Design goals

- protocol-driven rather than implementation-driven
- simple JSON payloads for AI-generated requests
- usable in cloud, distributed, and direct headless LAN deployments
- small enough to embed into agent runtimes without dragging UI dependencies

## First-cut capabilities

### Control plane

- `GET /api/health`
- `GET /api/v1/protocol`
- `GET /api/v1/protocol/agents`
- `POST /api/v1/fem/*/jobs`
- `GET /api/v1/jobs/:job_id`
- `POST /api/v1/jobs/:job_id/cancel`

### Solver RPC

- `ping`
- `describe_agent`
- `solve_bar_1d`
- `solve_truss_2d`
- `solve_truss_3d`
- `solve_plane_triangle_2d`
- `cancel_job`

## Intended AI use

For AI agents, the recommended flow is:

1. Query the control-plane protocol descriptor.
2. Inspect reachable agents or direct endpoints.
3. Generate a JSON payload for the desired FEM study.
4. Submit through the control plane or directly over solver RPC.
5. Poll and stream progress until completion.

The SDKs are deliberately thin wrappers over public contracts so higher-level AI
planning layers can stay language-agnostic.
