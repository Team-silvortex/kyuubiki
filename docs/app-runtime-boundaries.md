# App And Runtime Boundaries

This document defines the top-level product boundary for Kyuubiki.

Use this file as the primary source for product-role separation.
Do not duplicate detailed agent authority or headless transport rules here;
those belong to
[agent-control-authority.md](agent-control-authority.md),
[agent-orchestrator-boundary.md](agent-orchestrator-boundary.md), and
[headless-agent-contract.md](headless-agent-contract.md).

The hard rule is:

- frontend surfaces and runtime execution are architecturally decoupled
- `Hub` is the system entrypoint and workload shell
- `Workbench` owns concrete engineering workflow execution UX
- `Installer` owns deployment and runtime/agent lifecycle management
- `runtime` and `agent` layers execute work through stable protocols rather than
  UI coupling

If we keep this split clean, the product can grow without turning the frontend
into a hidden runtime controller or turning runtimes into UI-specific code.

## Four Product Roles

### `Hub`

`Hub` is the desktop-level system entrypoint.

It should own:

- entry into `Workbench`, `Installer`, and future system surfaces
- global workload visibility
- project launch and recent-work navigation
- runtime target overview
- health, logs, diagnostics, and version visibility
- cross-runtime switching and inspection

It should not own:

- detailed workflow editing semantics
- deployment-authoring internals
- solver execution internals

`Hub` manages the whole workstation picture. It does not become the runtime.

### `Workbench`

`Workbench` is the concrete engineering workflow surface.

It should own:

- project editing
- operator composition
- study setup
- workflow execution UX
- result inspection
- domain-oriented modeling interaction

It should not own:

- runtime installation policy
- agent deployment topology authoring
- platform-specific environment repair

`Workbench` is where the work happens, not where runtime fleets are installed.

### `Installer`

`Installer` is the deployment and runtime-management surface.

It should own:

- install / uninstall runtime components
- install / uninstall agents
- path policy and storage visibility
- update, repair, and integrity checks
- cleanup of old residues
- platform-specific deployment actions
- bootstrap of local, remote, and distributed runtime shapes

It should not own:

- day-to-day workflow authoring
- project editing
- solver-specific modeling UX

`Installer` is the system deployment plane, not the engineering work surface.

### `Runtime / Agent`

`runtime` and `agent` layers are execution layers.

They should own:

- protocol-described capabilities
- job execution
- data-plane behavior
- runtime state and heartbeat
- operator execution
- orchestration-facing or mesh-facing control contracts

They should not own:

- frontend layout assumptions
- Hub navigation assumptions
- Workbench component structure
- Installer UX assumptions

The runtime must be usable without inheriting the frontend's internal shape.

For the stricter runtime-side split between Rust agents and the Elixir control
plane, see [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md).
For the headless caller and direct-mesh contract, see
[headless-agent-contract.md](headless-agent-contract.md).

## GUI / Backend Service Separation

The GUI is a client surface, not the owner of the backend runtime.

This means Workbench and Hub must not assume that the control plane, mesh
gateway, or agent-compatible service is co-located with the WebView process.
They may use same-origin APIs as the default developer layout, but that is only
one deployment topology.

The frontend API layer therefore resolves backend targets through a transport
adapter:

- default: same-origin `/api/...`
- deployment default: `NEXT_PUBLIC_KYUUBIKI_API_BASE_URL`
- persisted override: `kyuubiki-workbench-api-base-url`
- temporary override: `?kyuubikiApiBaseUrl=...` or `?apiBaseUrl=...`

Workbench exposes the persisted override in `System / Settings / Routing` so
the active GUI-to-backend binding is visible instead of hidden in code.

All of those paths still speak the same public HTTP contract. A GUI can point at
an orchestrator, a mesh gateway, or another compatible service, and a headless
SDK can use the same routes without importing GUI internals.

The browser-side API client must stay thin:

- `apps/frontend/src/lib/api/core.ts` owns request transport, timeout, response
  parsing, backend target resolution, and token header attachment
- `apps/frontend/src/lib/api/backend-target.ts` owns GUI-to-backend target
  selection and validation
- `apps/frontend/src/lib/api/*-types.ts` files own public data contracts for
  project storage, runtime/agent state, and result/security payloads
- `apps/frontend/src/lib/api/index.ts` is an export facade only; API clients
  inside `lib/api` should import concrete contract files instead of importing
  the facade back into themselves
- `apps/frontend/src/lib/workbench/workbench-secrets.ts` owns per-page
  in-memory operator secrets
- Workbench helpers, model import/export code, and UI components must not be
  required just to construct a backend request

That split keeps the TypeScript API client usable in tests, WebView shells, and
future adapter layers without dragging the whole Workbench UI graph with it.

Workflow execution has one additional seam:

- `apps/frontend/src/lib/workbench/workflow-backend-service-core.ts` defines the
  GUI-facing workflow backend service contract and pure factory
- `apps/frontend/src/lib/workbench/workflow-backend-service.ts` binds that
  contract to the default orchestrated HTTP API implementation
- `apps/frontend/src/components/workbench/workflow/workbench-workflow-controller.ts`
  consumes the service contract instead of importing HTTP runtime clients
  directly

That seam is where future direct-mesh, mobile WebView, mock, or SDK-aligned
workflow backends should attach. UI components own interaction state and run
presentation; backend service implementations own catalog fetch, job submit,
and job polling transport.

Result inspection follows the same pattern:

- `apps/frontend/src/lib/workbench/result-backend-service-core.ts` defines the
  GUI-facing result chunk service contract and pure factory
- `apps/frontend/src/lib/workbench/result-backend-service.ts` binds that
  contract to the default orchestrated and direct-mesh HTTP chunk APIs
- result viewport controllers consume a result service and cache by backend id
  rather than hard-coding result URL families in viewport logic

This keeps large-result virtualization reusable across orchestra, direct mesh,
mobile remote backends, and tests.

The boundary rule is strict:

- UI state may describe intent, selected runtime target, and project context
- backend services own execution, scheduling, mesh membership, and agent state
- SDKs call backend contracts directly rather than driving Workbench controls
- no backend service may require Workbench component structure to be present

## Architectural Consequences

This split implies:

- `Hub` talks about workloads, targets, and surfaces
- `Workbench` talks about engineering tasks and workflows
- `Installer` talks about deployment, repair, update, storage, and cleanup
- runtimes and agents talk about capabilities, tasks, state, and protocols

So the frontend does not "contain" the runtime.

Instead:

- UI surfaces call stable APIs, RPC, manifests, and schemas
- runtimes expose protocol contracts
- agents remain replaceable execution peers

## Decoupling Rule

The most important decoupling rule is:

- frontends may select, observe, and command runtimes
- frontends must not define runtime architecture
- runtimes may describe capability and state
- runtimes must not depend on specific frontend implementation details

This applies equally to:

- browser workbench
- Hub desktop shell
- mobile WebView control clients
- installer GUI
- headless SDK callers
- orchestration services

## Mobile WebView Boundary

Mobile GUI support is a remote-control surface, not a runtime deployment mode.

iOS and Android WebViews may host Hub or Workbench UI, select a remote backend,
submit jobs, inspect workflow state, and observe agents through public APIs.
They must not host orchestra, run Rust agents, install runtimes, or assume that
`localhost` is the execution backend.

This makes the decoupling rule concrete: mobile support is possible because the
GUI calls stable backend contracts instead of embedding runtime ownership in the
frontend. See [mobile-gui-runtime-boundary.md](mobile-gui-runtime-boundary.md)
for the focused mobile contract.

## Monorepo Mapping

This product boundary maps into the repository like this:

- `apps/hub-gui`
  system entrypoint and global workload shell
- `apps/frontend`
  current browser workbench surface
- `apps/workbench-gui`
  native desktop shell for the workbench surface
- `apps/installer-gui`
  deployment and lifecycle management shell
- `apps/web`
  one control-plane runtime family
- `workers/rust`
  execution runtimes, agents, and solver data plane
- `sdks/*`
  peer interfaces for headless protocol-driven access

## Design Check

When deciding where something belongs, use this quick filter:

1. Is this about entering the system, switching workload context, or seeing the
   whole machine/runtime picture?
   Put it in `Hub`.
2. Is this about building, editing, or running an engineering workflow?
   Put it in `Workbench`.
3. Is this about install, deploy, repair, cleanup, update, or integrity?
   Put it in `Installer`.
4. Is this about execution capability, heartbeat, solve behavior, or protocol?
   Put it in `runtime / agent`.

If a feature seems to belong in two places, the usual answer is not to merge
the layers. The usual answer is to expose the same runtime capability through
different surfaces with different responsibilities.

For the practical do-not-cross checklist, see
[architecture-red-lines.md](architecture-red-lines.md).
