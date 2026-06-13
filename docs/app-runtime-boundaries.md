# App And Runtime Boundaries

This document defines the top-level product boundary for Kyuubiki.

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
- installer GUI
- headless SDK callers
- orchestration services

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
