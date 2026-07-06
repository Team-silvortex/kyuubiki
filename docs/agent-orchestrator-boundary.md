# Agent And Orchestrator Boundary

This document freezes the runtime-side boundary between:

- `solver agent`
- `orchestrator / control plane`
- `frontend surfaces`
- `headless SDK clients`

It exists to stop architecture drift, especially the common confusion between:

- "the agent is pure Rust"
- "the whole system is pure Rust"

Those are not the same claim.

## Hard Statement

In `tamamono 1.x`, the intended boundary is:

- `solver agent = Rust compute peer`
- `orchestrator = Elixir control plane`
- `frontend = UI shell and workflow surface`
- `SDKs = protocol clients, not hidden runtimes`

The Rust agent is the execution peer.
The Elixir orchestrator is the management and coordination peer.
Neither one should quietly absorb the other.

## Agent-Embedded Engine Rule

Every started agent owns one local engine instance.

That engine is part of the agent runtime, not a task object and not an
orchestrator-owned process. The distinction is:

- `agent`
  long-lived compute peer and RPC surface
- `engine`
  agent-embedded execution instance used to run assigned operator work
- `task`
  scheduled unit of work submitted manually, by SDK, by direct mesh, or by the
  bound orchestra
- `operator package`
  execution payload fetched from the bound orchestra library when required

The engine should be visible through runtime descriptors so operators can
inspect what is capable of executing a task. It should not make the agent a
second control plane.

## What The Agent Is

`agent` means the solver-side execution runtime.

It should own:

- FEM solve execution
- solver RPC serving
- job-local progress emission
- heartbeat and self-description
- peer-mesh participation
- one embedded engine instance per agent process
- operator execution on the compute side
- temporary operator package cache materialized for assigned work
- compute-local benchmarking support

It should be implementable and runnable without:

- React
- Next.js
- Phoenix
- Hub UI
- Installer UI

This is why the repository describes the agent/data plane as Rust.

## What The Agent Is Not

The agent is not:

- the desktop entrypoint
- the project browser
- the workflow editor
- the install/update surface
- the persistent control-plane authority
- the user-facing source of truth for workload history

If an execution peer needs one of those concerns, it must receive it through a
protocol contract rather than by inheriting product-layer logic.

## What The Orchestrator Is

`orchestrator` means the control-plane runtime family.

It should own:

- job submission and cancellation
- workflow graph intake
- persistence
- result windows and chunking
- agent registry and routing
- cluster-aware coordination
- control-plane security policy
- workflow/operator catalog delivery
- authoritative operator package resolution for its bound agents

It may talk to many agents.
It may be local or remote.
It is not the same thing as the Hub.

## Task And Operator Fetch Boundary

Tasks and execution engines are deliberately separate.

- a task may be assigned manually, by a headless SDK, through direct mesh, or by
  an orchestra scheduler
- an agent executes the task with its embedded engine
- in `orch_managed` mode, the agent fetches required operator packages from the
  operator library owned by its bound orchestra
- in `offline_mesh` mode, the task source may be manual or mesh-driven, but the
  agent still must not pretend to own a full authoritative operator library
- fetched packages may be cached only as visible, cleanable execution cache

This means a workflow run can move between scheduling modes without changing the
core engine model: the scheduling authority changes, but the agent-local engine
remains the execution boundary.

## Operator Description Vs Execution Program

Operator descriptions may be authored, indexed, or served by the Elixir control
plane. That does not make Elixir part of the compute-side execution ABI.

The boundary is:

- `operator descriptor`
  catalog metadata used for search, UI grouping, validation, package fetch, and
  workflow graph assembly
- `operator task IR`
  the orchestration envelope that binds one operator, one input artifact, config,
  dataset context, routing hints, and integrity metadata
- `operator execution program`
  the language-neutral program contract inside the task IR that an agent engine
  can execute

The execution program is the part analogous to LSP in the VS Code ecosystem:
the editor extension can be written in TypeScript, but the language server
interaction is protocol-shaped. In Kyuubiki, the control plane can be Elixir,
but the agent-facing execution structure is:

- schema: `kyuubiki.operator-execution-program/v1`
- runtime protocol: `kyuubiki.operator-execution/v1` or `kyuubiki.solver-rpc/v1`
- package reference: `orchestra://operator-package/<operator-id>`
- ABI: JSON input/config/output bindings
- entrypoint: protocol-visible operator id or solver method

Agent engines should treat this as the execution contract. They should not
depend on Phoenix routes, Elixir modules, or control-plane private function
names to run operator work.

Agent-native builtins are allowed only when they still enter through the same
TaskIR and execution-program envelope. For example, the Rust agent may execute a
library-managed transform such as `transform.evaluate_material_thermal_shock`
directly after digest verification. That is not a bypass around TaskIR; it is a
compute-side dispatch implementation for an operator whose package reference is
already represented as library-managed or agent-native. External operator
packages must still go through package resolution, integrity verification,
activation, dispatch, and result serialization stages.

## Dual-Mode Task Description

Task descriptions are allowed to be authored through more than one runtime.

The preferred product path is:

- Elixir control-plane descriptor authoring
- fast catalog iteration
- hot-reload-friendly pure-function transforms
- workflow graph assembly and validation close to the orchestrator

But this is not exclusive. Rust-native operator SDKs and external SDK clients
may also author task descriptors directly, as long as they emit the same
language-neutral task IR and execution program.

Directly authored descriptors must still carry the minimum executable identity:
`id`, `family`, `kind`, and an `execution.package_ref` bound to the same
operator id. Bypassing catalog lookup must not mean bypassing package identity.

Task IR integrity includes both a descriptor digest and a task digest. The
descriptor digest covers the operator snapshot; the task digest covers the
actual execution envelope fields, including descriptor authoring, input,
config, dataset context, runtime hints, and execution program. Agents and
orchestrators can use this to audit whether a task changed after construction.

Task IR therefore carries `descriptor_authoring` metadata:

- `mode`
  examples: `elixir_control_plane`, `rust_native`, `external_sdk`
- `runtime`
  examples: `elixir`, `rust`, `python`, `elixir_sdk`
- `source`
  examples: `workflow_operator_catalog`, `rust_operator_sdk`
- `hot_reloadable`
  a description-layer property, not an agent execution requirement
- `execution_language = language_neutral`
  the important invariant for agents

This keeps Elixir as the rapid authoring and orchestration layer without making
Elixir the only way to describe valid work.

## What The Orchestrator Is Not

The orchestrator is not:

- the agent
- the Hub
- the browser workbench
- the installer
- the only deployment mode

It is one runtime target family that product surfaces can manage.

## Product-Side Boundary

Use this section only for the runtime-facing consequences of product shells.
The primary owner of Hub / Workbench / Installer role separation is
[app-runtime-boundaries.md](app-runtime-boundaries.md).

Frontend and desktop surfaces may:

- inspect agent health
- select runtime targets
- submit jobs
- inspect workflow results
- observe topology
- choose orchestrated or direct-mesh execution modes

Frontend and desktop surfaces must not:

- define agent-internal file layout
- depend on private runtime module names
- assume process-tree structure
- encode solver implementation details as UI behavior
- become the real authority for runtime architecture

If a UI needs runtime behavior, that behavior must be exposed through:

- HTTP APIs
- solver RPC
- manifests
- schemas
- dataset contracts
- operator descriptors

For the headless transport and gateway-vs-runtime rulebook, use
[headless-agent-contract.md](headless-agent-contract.md).
For one-orchestrator-versus-offline-mesh binding rules, use
[agent-control-authority.md](agent-control-authority.md).

## Allowed Deployment Shapes

This boundary supports several valid shapes:

- `local workstation`
  frontend + orchestrator + local Rust agents
- `cloud control plane`
  frontend + orchestrator + database + remote agents
- `distributed control plane`
  frontend/orchestrator separated from remote Rust agents
- `headless peer mesh`
  Rust agents without Phoenix on the hot path

The existence of several shapes is exactly why the agent must not inherit UI
or orchestrator internals.

Use [system-overview.md](system-overview.md) for the broader system map and
[operations.md](operations.md) for operator procedures inside those shapes.

## Transitional Reality In This Repository

Some parts of the current repository are still transitional.

These are acceptable only as implementation bridges, not as permanent
architectural truths:

- Elixir-side bridge helpers that invoke Rust worker or CLI processes
- browser-side direct-mesh helper routes that still mediate agent access
- runtime management flows that are surfaced through Hub or Installer shells
- mock or compatibility adapters used to keep local iteration moving

Those pieces do not redefine the target boundary.

The target boundary remains:

- Rust owns compute-peer execution
- Elixir owns control-plane orchestration
- UI surfaces consume contracts

## Current Transitional Inventory

The following areas are the main known transitional bridges in the repository
today.

### `apps/web/lib/kyuubiki_web/workers/mock_worker_adapter.ex`

Current role:

- Elixir-side bridge that invokes Rust CLI worker flows for local iteration

Why it is transitional:

- the control plane is still mediating a worker-launch shape rather than only
  speaking to long-lived agent/runtime peers through stable protocol paths

Desired end state:

- orchestrator submits work to protocol-visible Rust agents
- local developer flows may still exist, but as explicit dev-mode adapters
  rather than as architecture-defining runtime paths

### `apps/frontend/src/app/api/direct-mesh/**`

Current role:

- Next.js server routes that help the browser workbench talk to Rust agents in
  direct-mesh mode

Why it is transitional:

- they still place a frontend-adjacent server layer between the UI surface and
  the pure headless agent shape

Desired end state:

- keep these routes only as deliberate product-owned gateway contracts
- require explicit deployment opt-in when they are exposed outside local
  workstation use
- do not let them become the hidden source of truth for runtime architecture
- preserve a parallel headless path where SDKs and non-UI callers can reach
  the same capability without inheriting frontend assumptions

### `apps/installer-gui/src-tauri/src/remote.rs`

Current role:

- desktop-side bootstrap and remote start helper for Rust agents

Why it is transitional:

- command construction and remote process launching are still tightly coupled to
  desktop-side management flows

Desired end state:

- keep desktop bootstrap as an operator convenience layer
- move long-lived runtime authority and execution semantics back behind stable
  manifests, installer contracts, and agent self-description

### Frontend runtime governance and dispatch helpers

Current role:

- frontend/runtime-mode selection, direct-mesh authority selection, and
  headless dispatch planning

Why it is transitional:

- parts of runtime policy are still described close to UI workflow logic

Desired end state:

- keep UI-side visibility and selection
- keep actual runtime authority, capability truth, and execution semantics in
  protocol-visible runtime descriptors and orchestrator contracts

## Convergence Priorities

Use this order when reducing transitional architecture over later `1.x` work.

1. keep agent capability truth in Rust
   The compute peer should remain the authoritative source for solver-side
   capability, health, and execution semantics.
2. keep orchestration truth in Elixir
   Scheduling, persistence, registry state, and control-plane security should
   not leak into frontend-owned logic.
3. convert bridges into explicit dev or operator layers
   If a bridge remains, name it as a bridge, scope it, and stop treating it as
   the permanent runtime model.
4. align SDK and UI surfaces to the same contracts
   If the browser can do something important, headless SDKs should be able to
   reach the same capability through public boundaries.
5. document every runtime-only authority clearly
   Package authority, agent authority, direct-mesh authority, and orchestrator
   authority should stay inspectable and explicit.

## Practical Review Checklist

Before accepting a runtime-facing change, confirm:

- the Rust agent remains runnable without Phoenix or React
- the orchestrator remains replaceable as one runtime target family
- the browser workbench is consuming a contract, not a private runtime detail
- Hub and Installer are managing runtime shape, not redefining it
- SDK callers can still reach the same capability through stable public
  boundaries
- any remaining bridge is explicitly marked as transitional, local-only, or
  operator-convenience behavior

## Red-Line Rules

Do not let the system drift into any of these states:

1. `agent` becomes UI-aware.
2. `orchestrator` becomes solver-hot-path compute.
3. `frontend` becomes the hidden runtime architecture authority.
4. `SDKs` become private escape hatches around public contracts.
5. `Hub` becomes the orchestrator itself.
6. `Workbench` becomes responsible for agent deployment internals.

Any of those should trigger architecture review, not opportunistic coding.

## Design Test

When adding a runtime feature, ask:

1. Could the Rust agent still run headlessly if this UI disappeared?
2. Could the orchestrator still coordinate remote agents if the desktop shell changed?
3. Is the behavior exposed through a named protocol or schema?
4. Are we adding capability, or leaking one layer into another?

If the answer to any of those is "no" or "not sure", the design is probably
crossing the boundary.

## Short Version

Use this sentence when people start mixing terms:

`The solver agent is Rust. The orchestrator is Elixir. The frontends are product shells. They meet through contracts, not inheritance.`
