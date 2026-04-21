# Architecture Overview

Kyuubiki is now organized as an engine-first monorepo with three deliberately
separated layers:

- `apps/frontend`
  Browser workbench and visualization layer
- `apps/web`
  Elixir control plane and persistence layer
- `workers/rust`
  Rust data plane and runtime tooling

The goal is to support local workstation runs, cloud control planes, and remote
solver clusters without forcing those modes to share unnecessary implementation
details.

The architectural style follows the shared principles in
[philosophy.md](/Users/Shared/chroot/dev/kyuubiki/docs/philosophy.md):
engine first, explicit boundaries, local-first development, and distributed
readiness.

## Runtime Layers

### Frontend Workbench

- modeling and editing workflows
- immersive 3D interaction
- project/material/version management
- chunk-aware large-result browsing
- two future-facing runtime modes:
  - `orchestrated_gui`
    Browser/UI connected to the Phoenix control plane for centralized,
    distributed clusters
  - `direct_mesh_gui`
    Browser/UI or desktop shell connected directly to headless Rust agents on a
    LAN without routing through Phoenix for solver coordination

### Orchestrator Control Plane

- job creation, lifecycle, cancellation, and watchdog handling
- persistence for projects, models, versions, jobs, and results
- health, deployment, and remote agent registry endpoints
- chunked result delivery
- agent routing and failover

### Rust Data Plane

- framed TCP solver agent transport
- protocol and engine-facing helpers
- FEM kernels and benchmark workloads
- installer CLI and deployment utilities

## Deployment Modes

The repository now explicitly supports:

- `local`
  Frontend + orchestrator + local Rust agents
- `cloud`
  Frontend + orchestrator + PostgreSQL-backed shared control plane
- `distributed`
  Centralized control plane with remotely deployed Rust solver nodes
- `direct_mesh`
  Frontend shell talking to a headless LAN solver mesh without a mandatory
  Phoenix coordinator on the hot path

Remote agents can be surfaced through:

- static endpoint lists
- manifest files
- runtime registration and heartbeat

## Contracts

Shared contracts live in `schemas/` and stay JSON-first:

- jobs
- progress events
- models
- material libraries
- project bundles
- agent manifests

This keeps the frontend, orchestrator, installer, and Rust runtime loosely
coupled and lets each side evolve independently behind stable payload shapes.

## Repository Shape

See [repository-structure.md](/Users/Shared/chroot/dev/kyuubiki/docs/repository-structure.md)
for the concrete directory layout and ownership boundaries.
