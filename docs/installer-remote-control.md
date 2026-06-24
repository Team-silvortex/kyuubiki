# Installer Remote Control Surface

Use this page as the source-of-truth note for the Installer remote deployment
and remote runtime control surface in the `tamamono 1.11.x` preparation line.

This document explains:

- what the Installer remote panel owns
- how remote-node actions are split across controllers
- how workflow snapshots are recorded and surfaced
- which behaviors are meant to stay operator-visible instead of implicit

## Why this surface matters now

The remote panel is no longer only a bootstrap helper.

It is becoming the operator-facing control surface for:

- remote bootstrap
- remote agent startup
- offline mesh preparation
- certificate alignment
- remote-node workflow trace review

That makes it part of the product boundary, not just a convenience form.

## Ownership boundary

The Installer remote surface owns:

- deployment-oriented remote node registration
- bounded remote host and workspace policy
- certificate inventory-driven node binding
- remote bootstrap and runtime launch actions
- mesh-preflight and mesh-rollout operator guidance
- node-local workflow snapshot inspection

It does not own:

- day-to-day modeling or result review
- workbench workflow authoring
- long-running runtime orchestration policy beyond Installer scope
- solver-side execution semantics

The intended split remains:

- `Hub`
  overall desktop shell and workload entry surface
- `Workbench`
  concrete engineering workflow and analysis surface
- `Installer`
  deployment, integrity, update, and remote runtime control surface
- `agents / orchestrator`
  runtime-side execution layers

## Current remote-panel structure

The remote panel is intentionally decomposed into smaller controllers instead
of one expanding file.

### Main coordinator

- `apps/installer-gui/ui/remote-node-panel.js`
  Assembles the remote-node subsystem, wires DOM events, and coordinates the
  controllers. It should stay a composition layer rather than a logic dump.

### Node renderer

- `apps/installer-gui/ui/remote-node-renderer.js`
  Groups and renders node cards without owning runtime mutation logic.

### Node executor

- `apps/installer-gui/ui/remote-node-executor.js`
  Owns node-level action execution for:
  - `probe`
  - `bootstrap`
  - `start`
  - `mesh-preflight`

It is also the place where node-level workflow snapshot writes are attached to
the action lifecycle.

### Recommended-action coordinator

- `apps/installer-gui/ui/remote-node-actions.js`
  Owns timeline follow-up actions such as:
  - focus mesh cluster id
  - focus peer endpoints
  - focus certificate controls
  - resolve node certificate state

### Bulk-action coordinator

- `apps/installer-gui/ui/remote-node-bulk-actions.js`
  Owns visible-node bulk dispatch for:
  - certificate assign / clear
  - mesh rollout
  - mesh preflight on filtered mesh nodes
  - generic visible-node action fan-out

### Certificate controller

- `apps/installer-gui/ui/remote-node-certificates.js`
  Owns certificate inventory matching, node binding state, focused certificate
  actions, and operator-visible certificate health summaries.

### Mesh controller

- `apps/installer-gui/ui/remote-node-mesh.js`
  Owns mesh diagnostics, cluster grouping, rollout-failure capture, retry
  handling, and cluster-local helper actions.

### Timeline controller

- `apps/installer-gui/ui/remote-node-timeline.js`
  Owns workflow snapshot presentation, semantic badges, latest-snapshot
  following, and the recommendation panel.

## Workflow snapshot contract

The Installer remote surface now records per-node workflow snapshots under the
remote node registry contract.

Current shape:

- `workflow_kind`
- `stage`
- `status`
- `summary`
- `recorded_at_unix_ms`
- `details`

The current implementation uses the shape for two purposes:

1. persist a short operator-facing execution timeline
2. provide a bridge between remote deployment actions and future durable
   workflow/asset lineage

The snapshot system is intentionally lightweight, but it should continue to
move toward the broader workflow asset model instead of becoming a private
Installer-only side format.

## Timeline behavior

The remote timeline is meant to behave like a control console, not a passive
log.

Current behavior includes:

- selecting a node card to inspect its recent snapshots
- following the latest snapshot when a new result arrives for the selected node
- semantic badges for workflow kind, stage, status, and diagnosis
- structured detail slots for key node identity fields
- recommendation items that can become direct actions or focus actions

This is important because the remote panel is now responsible for turning
runtime-side state transitions into something an operator can read and act on
without dropping into raw logs by default.

## Operator-visible behavior rules

The remote surface should preserve three rules.

### 1. Behavior must stay configuration-visible

Users should be able to see:

- allowed hosts
- allowed workspace roots
- cluster identifiers
- peer endpoints
- certificate bindings
- update and integrity paths

### 2. State should not depend on hidden residue

If remote actions leave behind local state, that state should move toward the
same explicit asset / contract posture as other durable product data.

### 3. Suggested recovery should stay actionable

Failure advice should prefer:

- direct retry actions
- focus-to-field guidance
- certificate-resolution guidance
- cluster/peer inspection guidance

instead of generic “something failed” messaging.

## Relationship to `1.11.x`

Inside the `1.11.x` to `1.20.x` industrialization range, this surface is
especially relevant to the `1.11.x` and `1.11.x` themes:

- `1.11.x`
  trust hardening, clearer operator-facing runtime claims, and fewer ambiguous
  control paths
- `1.11.x`
  asset and lineage formalization, where remote workflow snapshots should stop
  being isolated UI history and become part of a clearer execution record

## Related docs

- [app-runtime-boundaries.md](app-runtime-boundaries.md)
- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [headless-agent-contract.md](headless-agent-contract.md)
- [remote-pilot.md](remote-pilot.md)
- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)
- [installation-integrity-contract.html](installation-integrity-contract.html)
- [security.md](security.md)
