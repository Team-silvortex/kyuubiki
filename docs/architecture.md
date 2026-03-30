# Architecture Overview

This repository starts as a monorepo that keeps the orchestration layer, shared
contracts, and compute worker close together while allowing each part to evolve
independently.

## Layout

- `apps/web`
  Phoenix LiveView application boundary. This will own user-facing workflows,
  job submission, PubSub broadcasts, and orchestration state.
- `workers/rust`
  Rust workspace boundary for compute workers, protocol adapters, and CLI entry
  points.
- `schemas`
  Shared JSON Schemas for cross-process contracts. These act as the source of
  truth before we add code generation or protobuf bindings.
- `docs`
  Architecture notes and onboarding guides.
- `assets`
  Frontend-facing static assets and visualization fixtures.
- `scripts`
  Lightweight repository scripts that do not belong to either runtime.

## Runtime Responsibilities

### Phoenix LiveView

- Accepts mesh/material/boundary-condition uploads
- Persists job definitions and lifecycle state
- Pushes progress updates into PubSub and LiveView sessions
- Delegates compute to workers without embedding solver logic inside BEAM

### Rust Worker

- Pulls or receives executable job payloads
- Performs preprocessing, partitioning, solving, and postprocessing
- Emits throttled progress events and writes result artifacts
- Supports a local IPC transport first, then distributed TCP transport

## Contract Strategy

The first shared contracts live in `schemas/`:

- `job.schema.json` models the durable job record
- `progress-event.schema.json` models streamed status updates

These contracts intentionally stay transport-agnostic so we can map them to
JSON, MessagePack, or Protobuf later without changing the business meaning.

## Planned Next Steps

1. Generate a Phoenix app under `apps/web`
2. Create a Rust workspace manifest under `workers/rust`
3. Add a local transport prototype for job dispatch and streamed progress
4. Back shared schemas with validation tests in both stacks
