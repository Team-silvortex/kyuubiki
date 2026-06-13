# Agent Control Authority

This document defines the control-binding contract for distributed Kyuubiki
agents.

The rule is simple:

- one agent may be bound to exactly one `orchestra`
- or it may run in `offline_mesh` mode with no `orchestra`
- it must never authenticate to multiple orchestras at the same time

That rule needs to be visible in configuration, visible in APIs, and enforced in
runtime state.

## Control Modes

Every registered agent advertises one control mode:

- `orch_managed`
- `offline_mesh`

### `orch_managed`

In `orch_managed` mode:

- the agent belongs to one control-plane authority
- the authority is identified by `orch_id`
- an optional `orch_session_id` can pin the active authenticated session
- registration and heartbeat updates must not silently switch the bound
  orchestra

This is the normal mode for centrally scheduled deployments.

### `offline_mesh`

In `offline_mesh` mode:

- the agent is not attached to any orchestra
- `orch_id` must be absent
- `orch_session_id` must be absent
- the agent may participate only in offline or de-centered mesh behavior

This mode is for direct peer groups and offline deployment shapes where the
control plane is intentionally bypassed.

## Required Visible Fields

The runtime-visible binding contract is:

- `control_mode`
- `orch_id`
- `orch_session_id`

These fields do not all need to be editable in every UI, but they must be
inspectable so the operator can understand why an agent is accepted or rejected.

## Transition Rules

An agent may stay inside the same control authority:

- `orch_managed` -> same `orch_id`
- `orch_managed` -> same `orch_id` and same `orch_session_id`
- `offline_mesh` -> `offline_mesh`

An agent may not silently transition across authorities:

- `orch_managed(A)` -> `orch_managed(B)`
- `orch_managed` -> `offline_mesh`
- `offline_mesh` -> `orch_managed`

If the deployment really needs to switch authorities, the current binding should
be explicitly removed first, then re-registered under the new authority.

In practice that means:

1. unregister the existing agent binding
2. clear any old runtime association
3. register again under the new control mode or orchestra

## Why This Matters

Without exclusive control authority, the same agent can become an ambiguous
target:

- one orchestra thinks it owns scheduling rights
- another orchestra thinks it can dispatch into the same process
- the operator cannot reason about cache ownership, job ownership, or package
  fetch authority

That ambiguity is especially dangerous now that:

- operator libraries are centralized
- agents fetch operator packages on demand
- workflow dispatch may span multiple machines

The agent has to know who its authority is, or that it has no authority at all.

## Relation To Operator Library Centralization

This contract complements the central operator-library model:

- operator-library truth belongs to `orchestra`
- agents may fetch execution packages from that authority
- a single agent must not accept competing fetch authority from multiple
  orchestras

So the operator-library contract and the control-authority contract reinforce
each other:

- one authoritative library control plane
- one authoritative orchestra binding per agent

## Current Runtime Enforcement

The current registry-level guardrails are:

- new registrations are normalized to a visible control mode
- `orch_managed` registrations require an orchestra identity
- `offline_mesh` registrations reject orchestra fields
- heartbeat updates cannot silently rebind an already-registered agent to a
  different orchestra
- heartbeat updates cannot flip an agent between `orch_managed` and
  `offline_mesh`

That gives us a stable base for:

- agent/network UI inspection
- distributed scheduler rules
- operator package fetch policies
- future signed authority epochs if we choose to add them later
