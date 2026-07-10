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

GUI surfaces publish their runtime-facing capability expectations through
[`schemas/gui-runtime-capability-manifest.schema.json`](../schemas/gui-runtime-capability-manifest.schema.json).
The reference manifest is
[`schemas/examples.gui-runtime-capability-manifest.json`](../schemas/examples.gui-runtime-capability-manifest.json)
and uses `kyuubiki.gui-runtime-capability-manifest/v1`.
Product manifests live under
[`config/gui-runtime-capabilities`](../config/gui-runtime-capabilities).

That manifest records:

- which product-owned surface is speaking (`Hub`, `Workbench`, `Installer`,
  browser WebView, or mobile WebView)
- which runtime target kinds it can bind to (`orchestra`, `agent`, `mesh`,
  `direct_runtime`, `installer_runtime`, or `offline_bundle`)
- whether a surface is only binding to a runtime or is allowed to host/manage
  one; those are intentionally different capabilities
- whether each binding has headless SDK parity
- whether the same binding is safe for mobile WebView control clients
- which credential surface is allowed for that binding
- which degraded modes remain legal when no compatible runtime is reachable

The manifest is intentionally not an implementation registry. It is the
GUI-side capability contract that lets a surface choose a backend without
embedding backend topology in React, Tauri, or mobile WebView code.
GUI code should select bindings by declared capability such as
`workflow.submit`, `solver.submit`, or `result.chunk.read`; component code
should not branch directly on `orchestra`, `mesh`, or `installer_runtime`
unless it is implementing the shared binding adapter itself.

The browser-side API client must stay thin:

- `apps/frontend/src/lib/api/core.ts` owns request transport, timeout, response
  parsing, default backend target resolution, and injectable
  `WorkbenchApiRequestContext` request wiring
- `apps/frontend/src/lib/api/auth-context.ts` owns GUI runtime-mode and
  in-memory secret projection into governed auth headers
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
Non-default shells should provide their own request context instead of changing
the transport core.
- `apps/frontend/src/lib/api/runtime-client.ts` exposes
  `createRuntimeApiClient(request)` so runtime/workflow/status callers can keep
  the same API shape while using a shell-specific request context
- `apps/frontend/src/lib/api/security-results-client.ts` exposes
  `createSecurityResultsApiClient({ requestJson, requestText })` so security,
  export, and result-chunk callers can share the same injectable transport
  shape without importing UI code
- `apps/frontend/src/lib/api/project-client.ts` exposes
  `createProjectApiClient(request)` so project, model, and model-version
  persistence can use the same replaceable transport boundary as remote GUI and
  headless callers
- `apps/frontend/src/lib/api/headless-results-client.ts` exposes
  `createHeadlessResultsApiClient(request)` for headless result-record fetches
  that should not depend on Workbench component state
- `apps/frontend/src/lib/api/headless-handoff-client.ts` exposes
  `createHeadlessHandoffApiClient(request)` for orchestra handoff submit,
  status, history, and snapshot reads
- Workbench backend-service bindings use the runtime client instance
  (`defaultRuntimeApiClient` by default) instead of importing individual runtime
  request functions, so an alternate shell can swap the client composition point
- `apps/frontend/src/lib/workbench/backend-service-composer.ts` is the default
  composition point for runtime-backed Workbench services; alternate shells
  should create their service set from supplied runtime and result/security
  client instances

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

Study execution uses the same backend-service boundary:

- `apps/frontend/src/lib/workbench/study-run-backend-service-core.ts` defines
  the GUI-facing study run contract, direct-mesh/orchestrated routing, and job
  polling seam
- `apps/frontend/src/lib/workbench/study-run-backend-service.ts` binds that
  contract to the existing FEM HTTP submitters, direct-mesh solver endpoint,
  and FEM input payload resolvers
- `apps/frontend/src/components/workbench/workbench-run-controller.ts`
  consumes the service contract and keeps only UI state, precheck messaging,
  result display, and polling presentation logic

This prevents React controllers from becoming the only way to run a solver.
Headless SDKs, mobile WebView clients, and future mesh adapters can share the
same backend intent contract without importing Workbench component structure.

Job history and cancellation also sit behind a backend service:

- `apps/frontend/src/lib/workbench/job-history-backend-service-core.ts`
  defines the GUI-facing history and cancellation contract
- `apps/frontend/src/lib/workbench/job-history-backend-service.ts` binds that
  contract to the default HTTP job history and cancellation APIs
- `apps/frontend/src/components/workbench/workbench-job-history-controller.ts`
  consumes the service contract instead of importing job runtime clients

This keeps job administration usable from desktop GUI, mobile remote GUI, and
headless callers without requiring a React hook to own the transport.

Workbench administrative job/result editing has its own data boundary:

- `apps/frontend/src/lib/workbench/admin-data-backend-service-core.ts`
  defines the GUI-facing job/result administration contract
- `apps/frontend/src/lib/workbench/admin-data-backend-service.ts` binds that
  contract to the default HTTP job status/update/delete and result
  list/update/delete APIs
- primary action and admin-result controllers consume the service contract for
  history job open, job metadata edits, job deletion, result refresh, result
  edits, and result deletion

This keeps operator-facing administration usable without coupling Workbench
buttons directly to one WebView-local HTTP client. A remote GUI, test harness,
or headless adapter can provide the same job/result contract while preserving
the UI workflow. Project bundle export also reads completed job results through
this service, so portable project snapshots do not need a separate UI-only job
status client.

Security event read/write paths follow the same boundary:

- `apps/frontend/src/lib/workbench/security-event-backend-service-core.ts`
  defines the GUI-facing security event read/write contract
- `apps/frontend/src/lib/workbench/security-event-backend-service.ts` binds the
  contract to the default HTTP security event APIs
- data refresh and assistant audit controllers consume the service contract
  instead of importing security event runtime clients directly

This lets safety telemetry work across desktop GUI, mobile remote GUI, and
headless orchestrated callers while keeping local audit fallback behavior in
the UI layer.

Project library paths are also backend-service owned:

- `apps/frontend/src/lib/workbench/project-library-backend-service-core.ts`
  defines the GUI-facing project, model, and model-version CRUD contract
- `apps/frontend/src/lib/workbench/project-library-backend-service.ts` binds
  that contract to the default `ProjectApiClient`
- `apps/frontend/src/components/workbench/workbench.tsx` receives project
  actions from the project library backend service instead of importing project
  HTTP API functions directly
- data refresh controllers consume the service contract instead of importing
  project runtime clients directly
- project storage controllers consume the same service for bundle export
  details, project creation/update/delete, model save/delete, and model-version
  create/rename/delete
- script project/model automation consumes the same service so WASM Python,
  assistant plans, and recorded DSL actions cannot bypass the project library
  backend boundary

This stops the main Workbench project library path from threading raw backend
functions through component composition. Remaining project/model adapters should
converge on this service instead of growing parallel backend bindings.

Result inspection follows the same pattern:

- `apps/frontend/src/lib/workbench/result-backend-service-core.ts` defines the
  GUI-facing result chunk service contract and pure factory
- `apps/frontend/src/lib/workbench/result-backend-service.ts` binds that
  contract to the default orchestrated and direct-mesh HTTP chunk APIs
- result viewport controllers consume a result service and cache by backend id
  rather than hard-coding result URL families in viewport logic

This keeps large-result virtualization reusable across orchestra, direct mesh,
mobile remote backends, and tests.

Runtime status refresh also sits behind a service seam:

- `apps/frontend/src/lib/workbench/runtime-status-backend-service-core.ts`
  defines the GUI-facing runtime status snapshot contract and pure factory
- `apps/frontend/src/lib/workbench/runtime-status-backend-service.ts` binds the
  contract to orchestrated health/agent APIs and direct-mesh agent discovery
- data refresh controllers consume runtime snapshots instead of manually
  merging registry leases or synthesizing direct-mesh health inside UI code

This keeps health, agent, lease, and mesh discovery logic reusable across Hub,
Workbench, mobile remote-control clients, and tests.

The boundary rule is strict:

- UI state may describe intent, selected runtime target, and project context
- backend services own execution, scheduling, mesh membership, and agent state
- SDKs call backend contracts directly rather than driving Workbench controls
- no backend service may require Workbench component structure to be present

Headless execution follows the same rule. The browser-side runner in
`apps/frontend/src/lib/scripting/workbench-headless-execution.ts` accepts
`HeadlessExecutionBackendClients` so generated scripts, tests, and future SDK
bridges can supply runtime, project, and result clients without importing the
Workbench UI tree.
The Workbench headless workflow panel also talks through
`apps/frontend/src/lib/workbench/headless-workflow-backend-service.ts` so agent
discovery and orchestra handoff management stay behind replaceable runtime and
handoff clients.

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
