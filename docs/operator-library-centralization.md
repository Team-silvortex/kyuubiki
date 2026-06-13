# Operator Library Centralization

This document defines the intended `tamamono 1.x` rule for operator library
ownership in distributed agent deployments.

The short version is simple:

- there is one authoritative operator library
- that authority lives behind `orchestra`
- agents do not keep a full replicated copy of the operator library
- agents fetch operator packages only when a workflow run requires them

## Why this rule exists

Kyuubiki is not trying to build a peer-to-peer pile of drifting operator
installations.

The project goal is a controlled distributed runtime where:

- version authority is unambiguous
- operator upgrades happen once at the center
- agents remain lightweight
- residual local state stays visible and cleanable
- distributed execution does not create silent library forks

If every agent carries a full operator library copy, the system quickly picks
up the problems we explicitly want to avoid:

- version skew
- hidden leftovers
- hard-to-debug execution drift
- expensive cross-machine maintenance
- accidental execution against stale operator packages

## Core rule

The operator library itself must be centrally owned.

In practice that means:

- `orchestra` owns the authoritative operator registry
- `orchestra` owns the authoritative operator package store
- `orchestra` resolves operator identity, version, and integrity metadata
- agents execute operators, but do not define the operator library truth

Agents may keep temporary fetched packages, but that cache is not the library.

## Authority model

The authority chain should be:

1. workflow graph references `operator_id`
2. `orchestra` resolves that id against the central library
3. `orchestra` emits an execution manifest for the run
4. the selected agent fetches the required operator package
5. the agent verifies integrity, executes, and reports back

This keeps the library center-of-gravity in one place.

## Agent responsibilities

An agent should only need these operator-related capabilities:

- fetch operator package from `orchestra`
- verify package integrity
- materialize package into a temporary execution cache
- execute the package
- evict or clean temporary package state according to cache policy

An agent should not be responsible for:

- deciding canonical operator versions
- publishing authoritative operator metadata
- keeping a permanent full library mirror
- independently mutating operator definitions

## Cache policy

Agent-side operator caching is allowed only as an execution optimization.

The default expectation for `tamamono 1.x` should be:

- cache scope is explicit
- cache entries are attributable to fetched operator refs
- cache is disposable
- cache cleanup is visible to the user or operator
- cache is never treated as an independent library authority

Suggested cache scopes:

- `ephemeral`
  fetched for one execution stage and dropped immediately
- `job`
  reused only for the lifetime of one workflow job
- `session`
  reused during a short-lived agent session and then reclaimed

Full persistent agent-side library replication is not part of this model.

## Workflow runtime manifest additions

Distributed workflow execution should carry operator fetch metadata directly in
the runtime manifest, not as hidden scheduler state.

The runtime manifest should describe:

- `dispatch_policy`
  declares that authority is central and fetch-on-demand is required
- `operator_fetch_plan`
  one entry per required operator
- `placement_tags`
  hints for where an operator should run
- `required_capabilities`
  runtime features an agent must provide before execution

That lets the workflow package explain how distributed execution is expected to
behave.

## Fetch contract

The central library should be able to answer a request like:

- operator id
- requested version or version policy
- execution target metadata
- integrity expectations

And return:

- canonical package reference
- exact package version
- integrity hash or signature reference
- allowed cache scope
- placement/capability metadata

This is not just package distribution.
It is part of the operator execution contract.

## Interaction with the operator catalog

The operator catalog should eventually expose more than UI-facing descriptors.

It should grow to describe:

- execution mode
- central package reference
- required runtime capabilities
- placement hints
- whether an operator is orchestra-only or fetchable by agents

That does not require every field to be enforced immediately.
It does require the central-library model to be reflected in the descriptor
shape from now on.

## Non-goals for now

`tamamono 1.x` does not need all of the following before using this model:

- a public marketplace
- remote third-party operator installation
- fully dynamic agent-side plugin mounting
- peer-to-peer operator exchange between agents

The immediate goal is stricter central authority for built-in and trusted local
operator families.

## Current project direction

For the current codebase, this means:

- keep the authoritative operator registry in the web/orchestra control plane
- let workflow runtime manifests describe fetch-on-demand behavior
- keep agent selection in the distributed scheduler
- do not let individual agents silently become operator-library authorities

That matches the broader project direction:

- distributed agents
- composable workflows
- explicit contracts
- controlled installation and cleanup
