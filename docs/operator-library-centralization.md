# Operator Library Centralization

This document defines the intended `daji 3.x` rule for operator library
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

## Agent engine boundary

Every agent process starts with an embedded execution engine.

That engine is allowed to run assigned operator tasks. It is not allowed to
become a local authoritative operator library. The durable ownership stays with
the bound `orchestra`; the agent-local engine only receives a task, fetches the
needed package, verifies it, executes it, and reports results.

This is the key separation:

- scheduler decides which task runs where
- bound `orchestra` resolves operator packages
- agent-embedded engine executes the resolved package
- temporary cache helps performance but is not a library replica

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

The default expectation for `daji 3.x` should be:

- cache scope is explicit
- cache entries are attributable to fetched operator refs
- cache is disposable
- cache cleanup is visible to the user or operator
- cache is never treated as an independent library authority

Suggested cache scopes:

- `ephemeral`
  fetched for one execution stage and dropped immediately
- `job`
  reused only for the lifetime of one workflow job and released through its
  explicit terminal RPC
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

The first executable contract is now present:

- publish-time index: `kyuubiki.operator-package-distribution/v1`
- resolve response: `kyuubiki.operator-package-resolution/v1`
- target identity: `<os>-<architecture>`, for example `linux-x86_64`
- authority mode: `bound_orchestra`
- cache scope: `task_required_disposable`

The self-hosted distribution root is configured with
`KYUUBIKI_OPERATOR_PACKAGE_DISTRIBUTIONS`. Orchestra validates the static index,
package manifest, regular-file layout, byte sizes, and SHA-256 digests before it
returns canonical same-origin download paths. Installer then repeats identity,
size, digest, and package-readiness checks before atomic activation. A target
miss is explicit; another platform is never substituted.

This now closes both the explicit Installer pull path and the Agent TaskIR
handoff. An orchestrated Agent may start with an empty managed `packages`
directory. On `execute`, an admitted `orchestra_fetch` TaskIR is resolved only
against that Agent's configured Orchestra, downloaded with its in-memory cluster
token, verified, atomically installed, and loaded before dispatch.
Preflight remains side-effect free. Concurrent cache misses are serialized, and
an already verified package produces a `verified_cache_hit` receipt instead of
another download.

Every miss is assembled in an owner-marked Agent generation using the same native
Installer verification and atomic-install APIs. A same-id newer version replaces
the old package only in the candidate generation. After complete dynamic-host and
TaskIR identity validation, host and runtime binding switch together. In-flight
tasks retain the previous host, and a weak-reference reaper deletes its generation
only after the final task releases it. The active cache remains visible under
`<store>/agent-runtime-generations` and may be reused for the Agent process
lifetime.

Every process holds an exclusive lease in its own cache session. A later Agent
startup removes only owner-marked sessions whose lease can be exclusively
acquired, skips sessions still owned by live peers, and retains malformed entries
fail-closed. Abrupt termination therefore leaves recoverable cache state rather
than permanent residue, without allowing one Agent to delete another Agent's
loaded library. Unleased pre-session cache entries are reported as invalid rather
than deleted speculatively. The generation execution receipt exposes removed,
active, and invalid session counts.

`cache_scope: none` now proves immediate task-scope deletion: after dispatch the
Agent activates an owner-marked generation that excludes the requested package,
reports `evicted_after_execution`, and refetches on later demand. Host leases keep
the previous dynamic library alive for concurrent work until its final reference
is released.

`cache_scope: job` now requires a top-level execute RPC `job_id`. Once all work
for that identity is terminal, Orchestra calls `release_operator_package_job`.
The Agent removes that owner, preserves packages shared by another job or a
durable scope, and evicts all exclusively owned packages in one generation
switch. Repeating the call returns `already_released`; `cancel_job` routes through
the same cleanup path. This lifecycle is locally qualified with a real Agent and
Orchestra package server, including shared ownership and refetch after final
release. Real two-host central acquisition is now retained under
`releases/usability-evidence/2.19.0/operator-package-acquisition-operational-qualification.json`:
a macOS Elixir Orchestra serves the only package copy to an Installer-managed
Linux Agent, two disposable tasks both refetch after eviction, and cleanup leaves
no active package or managed run root. Current stable-ABI Windows requalification
and installed desktop operation on all three platforms remain open.

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

## Interaction With External Stores

An external store is an upstream distribution surface. It is not the agent-side
operator-library authority.

The intended chain is:

- An external store can host reviewed operator packages and workflow templates.
- A self-hosted or hosted Kyuubiki control plane can sync, mirror, or select
  approved resources from that upstream source.
- The bound `orchestra` remains the local authority that resolves the operator
  package for an actual workflow run.
- Agents still fetch on demand from their bound `orchestra`, not directly from
  a marketplace account session.

This keeps deployment-owned distribution and access policy separate from runtime
authority. Operator identity, version selection, dispatch manifests, cache
policy, provenance, and execution verification remain local runtime contracts.

## Managed operator modules

The catalog now treats every operator as part of a managed module.

The module id is intentionally simple:

- `<domain>.<kind>`
- examples: `mechanical.solver`, `thermal.solver`,
  `electromagnetic.workflow_bridge`

Each operator descriptor carries a `module` block for:

- UI grouping
- catalog filtering
- deployment and cache policy visibility
- future package ownership and integrity checks

The first managed fields are:

- `module.id`
  stable query and grouping key
- `module.label`
  user-facing group name
- `module.lane`
  broad workflow lane such as `physics`, `coupling`, `dataflow`, or `delivery`
- `module.operator_scope`
  operator execution scope such as `physics`, `coupling`, or `inspection`
- `module.management`
  central-library policy for agent fetch, cache, and UI grouping

This keeps large operator catalogs from becoming a flat list.
It also gives Workbench, Hub, headless SDKs, and future installer tooling the
same vocabulary for deciding where an operator belongs.

## Non-goals for now

`daji 3.x` does not need all of the following before using this model:

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
