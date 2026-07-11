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

### Layer 2.5: operator execution program

Operators may be described and indexed by the Elixir control plane, but the
agent-facing executable structure is language-neutral.

The task IR carries:

- `execution_program.schema_version = kyuubiki.operator-execution-program/v1`
- `runtime_protocol = kyuubiki.solver-rpc/v1` for direct solver methods
- `runtime_protocol = kyuubiki.operator-execution/v1` for packaged operators
- JSON ABI bindings for input artifact, config, and output artifact
- a protocol-visible entrypoint, never an Elixir module/function name

The matching schema lives at
[`schemas/operator-execution-program.schema.json`](../schemas/operator-execution-program.schema.json).

This is the LSP-like layer: product and catalog code can be implemented in one
language, while the agent engine runs a stable protocol object.

Rust agents expose packaged operator submissions through
`run_operator_task_ir`. The RPC handler is only the transport wrapper; native
TaskIR validation, digest checking, and summary construction belong to
`workers/rust/crates/cli/src/operator_task_runtime.rs` so package fetching and
dispatch can attach without reshaping the solver RPC surface.
The shared Rust protocol crate exposes a checked summary API for this path so
runtime callers receive structured pre-execution error codes instead of parsing
human-readable strings.

Pre-execution failures are classified before package fetch or solver dispatch:
`operator_task_digest_missing`, `operator_task_digest_mismatch`,
`operator_task_digest_invalid`, `operator_task_mirror_mismatch`,
`operator_task_execution_abi_mismatch`, `operator_task_program_mismatch`, and
`operator_task_entrypoint_mismatch`. These are task/contract failures, not
solver-kernel failures.

Task description is dual-mode:

- Elixir control-plane authoring is the default and remains the fastest path for
  catalog iteration and hot-reload-friendly workflow logic.
- Rust-native SDK authoring is valid when runtime-side packages need to emit
  task IR directly.
- Other SDKs may author task IR if they preserve the same schema and execution
  program contract.

Agents should treat `descriptor_authoring` as audit metadata. They execute the
language-neutral `execution_program`, not the authoring runtime.

### Layer 3: GUI gateway and handoff payloads

These are useful product layers, but they are not the runtime source of truth.

Examples:

- `apps/frontend/src/app/api/direct-mesh/**`
- `kyuubiki.headless-orchestra-handoff/v1`

These layers may package policy, defaults, or operator context, but they
should compile down to stable control-plane or solver-RPC requests.

## Authority rules

Use this section for headless-runtime consequences of authority, not for the
full binding-state rulebook.

Headless agents must follow these invariants:

- one agent may be bound to at most one orchestrator authority at a time
- an offline peer-mesh agent must not also present itself as orchestrator-bound
- agent package/library authority remains centralized rather than replicated
  independently on each node
- SDK callers may choose direct solver RPC or control-plane mediation, but must
  not mix both authorities in one implicit submission path

The primary ownership of binding modes, visible fields, and legal transitions
lives in [agent-control-authority.md](agent-control-authority.md).

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
- SDK -> control plane HTTP `POST /api/v1/operator-tasks/prepare`
  Best when a headless client needs the control plane to verify TaskIR
  integrity and expose the language-neutral execution summary before dispatch.
- SDK or Orchestra -> solver RPC `run_operator_task_ir`
  Best when a trusted caller wants the Rust agent to validate TaskIR natively.
  Current agent behavior is staged preflight: it verifies digest and execution
  summary, returns an `execution_plan`, and reports `fetch_package` as blocked
  only while the operator package runtime is detached.
  Request params accept `mode: "preflight"` by default and `mode: "execute"`
  for future dispatch. Attached package runtimes now move `fetch_package` to
  pending and expose `next_stage: "fetch_package"` while engine dispatch wiring
  is still being completed. The Elixir control-plane client forwards this mode through
  `AgentClient.run_operator_task_ir(task_ir, mode: :execute)`.

The preflight result also exposes an `operator_package_runtime` contract. Until
the runtime is attached it reports `status: "not_attached"` and names the
expected host as `kyuubiki-engine.operator-sdk-host/v1`. That contract mirrors
the Rust operator SDK host policy: packages are fetched on demand, activated
through the operator registry, restricted to `rust_crate` runtimes by default,
and must keep entrypoints inside the package root.
The same result includes `package_fetch_request`, a language-neutral request
envelope with schema `kyuubiki.operator-package-fetch-request/v1`. Detached
agents report `request_status: "blocked_runtime_not_attached"`; attached agents
report `request_status: "ready_to_resolve"` and include the target host id plus
packages root. This is the handoff object for future orchestra package catalog,
mesh cache, or installer-managed package host integrations.
Callers should use `operator_package_runtime_ready` as the coarse readiness bit
and read `operator_package_runtime.status` for the detailed reason. The agent
runtime already models both `not_attached` and `attached` host bindings; an
attached host reports its host id, package root, and activated package count,
but execution still waits for package fetch and dispatch wiring. `blocked_stage`
is `null` in this attached state so callers can distinguish "missing runtime"
from "runtime ready, next stage pending".
New callers should prefer the machine-readable `execution_readiness` object for
automation decisions. It reports `status`, `current_stage`, `blocking_stage`,
`blocking_reason`, `blocking_owner`, `required_action`, and `ready_to_dispatch`
without requiring callers to infer control flow from display-oriented text. Each
`execution_plan` stage also carries a `gate` value such as `passed`, `blocked`,
`open`, `waiting_for_fetch`, `waiting_for_integrity`, or
`waiting_for_dispatch`, so UI, SDKs, and orchestration logic can agree on the
same staged package-backed execution boundary.
Attachment is local-agent controlled, not request controlled: the agent process
can be started with `--operator-package-host-id`, `--operator-packages-root`,
and `--operator-activated-package-count`, or the matching
`KYUUBIKI_OPERATOR_*` environment variables.
The same snapshot is exposed through `describe_agent` and registration payloads
so the control plane can route package-backed operator tasks intentionally.
- SDK -> solver RPC
  Best when a trusted environment wants the shortest path to a specific solver
  node or LAN mesh.

### Product-owned bridge paths

- Workbench -> Next.js `direct-mesh` gateway -> solver RPC
  Valid for GUI-led operator flows, but still a gateway layer.
- Workbench -> headless handoff registry -> SDK or runtime executor
  Valid for workflow export and deferred execution, but not itself the solver
  protocol.

For operator-facing deployment and runtime procedures around these paths, use
[operations.md](operations.md) and
[installer-remote-control.md](installer-remote-control.md).

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
