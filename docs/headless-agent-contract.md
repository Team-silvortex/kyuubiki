# Headless Agent Contract

This document defines the runtime-facing contract for `kyuubiki-rust-agent`
when it is used as a headless compute-plane program.

Use it to keep SDKs, orchestrators, and future agent-network features aligned
without inheriting frontend-specific assumptions.

## Why this exists

Kyuubiki now has three different shapes that can touch solver execution:

- the browser workbench
- the orchestrator control plane
- headless SDKs and agent-network callers

Those shapes must not share authority accidentally.

The headless agent contract exists to make one thing explicit:

- a Rust solver agent is a standalone runtime program with its own public
  machine contract
- frontend `direct-mesh` routes are a product-owned gateway, not the runtime
  source of truth
- handoff envelopes are orchestration payloads, not the lowest-level agent
  protocol

## Stable runtime boundary

The stable runtime contract for headless agents is:

- program: `kyuubiki-rust-agent`
- role: `solver_agent`
- transport: `TCP`
- framing: `length_prefixed_u32`
- encoding: `JSON`
- protocol: `kyuubiki.solver-rpc/v1`

The canonical descriptor source is:

- [protocols.md](protocols.md)
- [apps/web/lib/kyuubiki_web/protocol.ex](../apps/web/lib/kyuubiki_web/protocol.ex)

Everything else should layer on top of that boundary rather than replacing it.

## Contract layers

### Layer 1: solver RPC

This is the actual runtime contract exposed by headless agents.

It includes:

- `ping`
- `describe_agent`
- `cancel_job`
- solver methods such as `solve_truss_3d`, `solve_frame_3d`,
  `solve_heat_plane_quad_2d`, and other declared FEM study entries

This layer should remain valid whether the agent is:

- launched by an orchestrator
- running as a standalone LAN peer
- reached directly by a headless SDK

### Layer 2: control-plane mediation

The orchestrator may schedule, persist, and route work on top of solver RPC.

That mediation is allowed to add:

- job lifecycle state
- persistent result storage
- cluster membership and health policy
- workload routing and failover

But it must not redefine solver payload semantics behind a private backend-only
 shape.

### Layer 3: GUI gateway and handoff payloads

These are useful product layers, but they are not the runtime source of truth.

Examples:

- `apps/frontend/src/app/api/direct-mesh/**`
- `kyuubiki.headless-orchestra-handoff/v1`

These layers may package policy, defaults, or operator context, but they
should compile down to stable control-plane or solver-RPC requests.

## Authority rules

Headless agents must follow these invariants:

- one agent may be bound to at most one orchestrator authority at a time
- an offline peer-mesh agent must not also present itself as orchestrator-bound
- agent package/library authority remains centralized rather than replicated
  independently on each node
- SDK callers may choose direct solver RPC or control-plane mediation, but must
  not mix both authorities in one implicit submission path

These rules match the current project direction:

- `single_orchestrator`
- `offline_mesh`
- no multi-orchestrator dual registration
- no hidden frontend ownership of runtime authority

## Required descriptor shape

Every headless agent should self-describe enough runtime information for
callers to make safe routing decisions.

Current required concepts:

- `program`
- `role`
- `protocol`
- `runtime`

Current important runtime fields:

- `runtime_mode`
  - `standalone`
  - `orchestrated`
  - `peer_mesh`
- `headless`
- `cluster_id`
- `peers`
- `health_score`
- `methods`
- `capabilities`

The exact JSON may evolve, but these concepts should remain visible so SDKs and
orchestrators can reason about the same node without reading implementation
internals.

## Submission matrix

Use these paths intentionally.

### Preferred headless paths

- SDK -> control plane HTTP
  Best when persistence, governance, catalog selection, or distributed routing
  matters.
- SDK -> solver RPC
  Best when a trusted environment wants the shortest path to a specific solver
  node or LAN mesh.

### Product-owned bridge paths

- Workbench -> Next.js `direct-mesh` gateway -> solver RPC
  Valid for GUI-led operator flows, but still a gateway layer.
- Workbench -> headless handoff registry -> SDK or runtime executor
  Valid for workflow export and deferred execution, but not itself the solver
  protocol.

### Anti-patterns

- building new runtime-only features that exist only behind frontend routes
- treating handoff envelopes as the canonical solver payload contract
- mixing orchestrator credentials and direct-mesh credentials in one hidden
  request path
- making agent-network behavior depend on browser-local state

## What should stay frontend-specific

The following concerns belong to product shells rather than runtime contracts:

- panel layout
- workbench automation selectors
- browser token storage policy
- operator approval prompts
- snapshot history and UI state restoration

Those features matter, but headless callers must not need them to use solver
capabilities.

## What should stay runtime-visible

The following concerns should remain visible to SDKs, orchestrators, and
headless agents alike:

- protocol version
- supported solve methods
- agent identity
- runtime mode
- cluster and peer visibility
- liveness and progress events
- cancellation behavior
- machine-readable errors

## Current transitional pieces

These are still useful, but should be treated as transitional packaging layers:

- [apps/frontend/src/lib/scripting/workbench-headless-orchestra-handoff.ts](../apps/frontend/src/lib/scripting/workbench-headless-orchestra-handoff.ts)
- [apps/frontend/src/app/api/v1/headless/handoff/route.ts](../apps/frontend/src/app/api/v1/headless/handoff/route.ts)
- [apps/frontend/src/lib/direct-mesh/rpc.ts](../apps/frontend/src/lib/direct-mesh/rpc.ts)

They should continue converging toward:

- thinner packaging
- less frontend-owned runtime knowledge
- more reuse by SDKs and runtime-side executors

## Review checklist

When a change touches agents, SDKs, or orchestration, verify:

1. Does the new behavior map cleanly to `kyuubiki.solver-rpc/v1` or
   `kyuubiki.control-plane/http-v1`?
2. If a GUI route is involved, is it acting as a gateway instead of becoming a
   hidden protocol owner?
3. Can a headless SDK or runtime caller use the same capability without
   browser-local state?
4. Are authority rules still explicit: one orchestrator or offline mesh, but
   not both at once?
5. Are identity, methods, and runtime-mode fields still visible through public
   descriptors?

If the answer to any of these is no, the change is probably re-coupling the
runtime to a product shell.
