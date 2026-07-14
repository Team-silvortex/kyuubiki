# tamamono 1.9 to 1.20

Use this page as the industrialization map for the second half of the
`tamamono 1.x` line.

This document is not the broad product north star. For that, use
[fem-blender-roadmap.md](fem-blender-roadmap.md).

This document is also not a single-release hygiene checklist tied to one older
minor-line transition.

It is the staged roadmap for turning the current platform skeleton into a
durable industrial product before `2.0`.

## Why this range exists

`tamamono 1.8.x` is already beyond the point where progress should be measured
mainly by feature count.

The primary job of `1.9.x` through `1.20.x` is to make the existing product
shape harder, more trustworthy, and more survivable:

- solver claims should become benchmark-backed trust claims
- workflows should become long-lived engineering assets
- agent, mesh, SDK, and workbench paths should converge on one runtime language
- install, update, integrity, and cleanup behavior should become predictable
- release quality should stop depending on memory and heroics

Treat this whole range as:

`the industrialization and boundary-hardening phase of tamamono 1.x`

## What should define progress in this range

The default win condition is not:

- more panels
- more one-off feature slices
- more speculative operator families

The default win condition is:

- stronger numerical confidence
- stronger task and runtime reliability
- stronger asset lineage and recovery
- stronger cross-surface contract stability
- stronger deployment, update, and safety discipline

## Line-wide rules

Every minor in this range should preserve four rules.

### 1. Prefer hardening over widening

If a change widens scope but weakens validation, it is probably early.

### 2. Keep UI, SDK, and runtime semantics aligned

The workbench should not carry private meaning that headless SDKs or agents do
not understand.

### 3. Treat workflow and asset data as durable product language

Workflow graphs, dataset contracts, snapshots, templates, reports, and result
bundles should move toward long-lived asset status, not temporary payload
status.

### 4. Use `2.0` only after the boundaries stop moving

`2.0` should mean:

- core contract surfaces are stable enough to explain cleanly
- runtime and product boundaries are no longer in active churn
- the industrial baseline is real, not aspirational

## The staged minor plan

### 1.9.x

Primary theme:
task-system industrialization

Focus:

- define a formal job lifecycle instead of treating solve actions as simple
  button events
- unify queueing, cancellation, timeout, retry, cleanup, and failure handling
- make task logs and result indexing visible across orchestrated, direct-mesh,
  and headless flows

What should be true before moving on:

- failed work does not silently leave dirty runtime state behind
- users can tell the difference between queued, running, failed, partial,
  cancelled, and recoverable work
- runtime logs are readable enough to explain high-signal failures quickly

What not to optimize for here:

- speculative new study families
- UI polish that hides task ambiguity instead of fixing it

### 1.10.x

Primary theme:
accuracy hardening, first major pass

Focus:

- turn supported solver families into benchmark-backed claims
- expand the current baseline plan in
  [accuracy-plan.md](accuracy-plan.md)
- add explicit benchmark case, reference metrics, tolerance, and automation
  status for each core family
- align operator-facing docs and control surfaces so trust claims, remote
  runtime behavior, and verification posture say the same thing

What should be true before moving on:

- major solver families have automated regression baselines
- drift between versions is discoverable instead of anecdotal
- support statements match real verification posture

What not to optimize for here:

- calling a sample or demo path “verified” without a real benchmark contract

### 1.11.x

Primary theme:
engineering-asset formalization

Focus:

- treat workflow, snapshot, template, result, and report data as formal assets
- add clearer lineage, restore, compare, import/export, and cleanup rules
- reduce dependence on hidden or ad hoc local state

What should be true before moving on:

- a result can be traced back to its workflow, inputs, and execution context
- project-local reusable assets feel intentional rather than incidental
- cleanup behavior is predictable and user-visible

What not to optimize for here:

- new temporary storage paths that bypass the emerging asset model

### 1.12.x

Primary theme:
workflow-language hardening

Focus:

- freeze workflow graph, dataset contract, and port semantics more aggressively
- remove temporary overlap and payload ambiguity between product layers
- align frontend, SDK, and runtime interpretation of the same workflow data

What should be true before moving on:

- the same workflow graph means the same thing across workbench, SDK, and agent
  surfaces
- cross-operator contracts survive export, import, and replay more cleanly
- temporary payload shortcuts are reduced instead of growing

What not to optimize for here:

- local convenience fields that make the shared contract harder to trust

### 1.14.x

Primary theme:
physics coverage before engine-format freeze

Focus:

- cover the major structural, thermal, electric, magnetic, acoustic, modal,
  nonlinear, contact, CFD-like, and coupled thermo-structural families at
  least at smoke level
- make the `physics-coverage` benchmark matrix the broad lane for proving that
  built-in solver families still have real execution paths
- keep headless SDK usage and frontend wasm Python responsibilities clearly
  split while ensuring solver paths do not depend on hidden frontend behavior
- collect enough varied physics examples to design `1.15.x` operator SDK
  contracts and `1.17.x` executable task files without overfitting to one
  solver family

What should be true before moving on:

- all built-in benchmark templates are reachable through `physics-coverage`
- coverage labels distinguish smoke, baseline, review, and qualification
- TaskIR and executable task design have examples from scalar fields, vector
  fields, coupled flows, modal outputs, nonlinear/contact solves, and
  fluid-like fields
- SDK, workbench, and agent paths agree on the same solver-family names

What not to optimize for here:

- calling broad smoke coverage “industrial validation”
- freezing executable task fields before enough physics families have gone
  through the same path
- treating headless flows as test-only shims or frontend automation as runtime
  semantics

### 1.15.x

Primary theme:
engine and operator-SDK industrialization

Focus:

- turn the `1.14.x` coverage set into a smaller engine-facing execution
  contract
- mature the Rust operator SDK and its descriptor model
- standardize operator capability declaration, validation posture, and error
  surfaces
- reduce the amount of privileged one-off wiring needed for new operators

What should be true before moving on:

- a new operator can enter the system through a repeatable contract-first path
- built-in and extension operators are closer to one shared integration model
- operator compatibility can be described rather than guessed

What not to optimize for here:

- hidden internal-only operator pathways that the ecosystem can never follow

### 1.17.x

Primary theme:
executable task files and post-processing depth

Focus:

- define the portable executable task file shape for solver-agent execution
  using the `1.14.x` physics coverage set as examples
- deepen result review, comparison, export, and analysis ergonomics
- strengthen the path from model intent to study setup to result inspection
- improve the rendering and interaction layer only where it supports real FEM
  use

What should be true before moving on:

- post-processing is more than a thin shell around raw payload display
- users can compare and inspect results more naturally
- rendering work supports engineering use instead of only visual style

What not to optimize for here:

- visual flourish that does not deepen real engineering review

### 1.17.x

Primary theme:
install and update-system convergence

Focus:

- unify update source selection, download, validation, staging, apply, and
  rollback behavior
- formalize standard install paths and disk-use rules across platforms
- keep cleanup and old-residue removal visible and reliable

What should be true before moving on:

- failed updates can be understood and recovered from
- install and cleanup rules are visible enough to avoid silent residue burdens
- platform differences are managed intentionally instead of drifting

What not to optimize for here:

- platform-specific exceptions that weaken the standard install contract

### 1.18.x

Primary theme:
security and audit reinforcement

Focus:

- deepen action auditing for sensitive runtime and deployment flows
- harden token, certificate, and remote-control boundaries
- clarify trust levels for agents, operators, and automation surfaces

What should be true before moving on:

- remote or privileged actions are traceable
- sensitive configuration is not mixed casually into normal project data
- security posture is reflected in real product behavior, not only in docs

What not to optimize for here:

- hidden trust assumptions around remote deployment or long-lived credentials

### 1.19.x

Primary theme:
release-discipline and ecosystem preparation

Focus:

- align docs, tests, release checklists, and version contracts
- elevate operator compatibility and validation posture into release-facing
  surfaces
- make the project easier for new contributors, teammates, and future models to
  orient around

What should be true before moving on:

- release readiness no longer depends heavily on manual memory
- documentation and shipping behavior stay visibly aligned
- the ecosystem story is easier to explain without caveats

What not to optimize for here:

- last-minute feature additions that weaken release predictability

### 1.20.x

Primary theme:
final pre-`2.0` boundary freeze and `tamamono 1.x` closeout

Focus:

- freeze the core contract, asset, runtime, and operator-boundary surfaces
- run one system-wide audit of what is truly stable enough for `2.0`
- explicitly decide what belongs in `2.0` and what should wait for `2.1+`
- avoid starting a `1.21.x` line unless a narrow stabilization emergency makes
  it unavoidable

What should be true before moving on:

- core product boundaries can be described cleanly without heavy caveats
- major workflow and runtime semantics are no longer shifting every cycle
- `2.0` can mean maturity instead of “another rewrite starts now”
- remaining unfinished work has an explicit destination: `moxi 2.0.0`,
  post-`2.0`, experimental, or retired

What not to optimize for here:

- late structural experiments on the critical path to the new major version

## Cross-range priorities

Across all of `1.9.x` through `1.20.x`, the three most important long threads
should be:

1. numerical trust
2. runtime reliability
3. durable engineering assets

If a proposed feature does not strengthen at least one of those three threads,
it probably belongs after the industrial baseline is stronger.

## Commercial Posture: `2.0` Is The First Trust Line, Not The Final War

`2.0` should be treated as the first credible commercial line, not as the point
where Kyuubiki already tries to defeat every incumbent industrial FEM suite.

The commercial promise for `2.0` should be:

`a trustworthy early commercial / research-partner platform for programmable,
workflow-driven FEM work`

That means `2.0` must be good enough for selected early customers, research
partners, internal R&D teams, and automation-heavy users to trust the system
for bounded real work.

It does not need to claim full parity with mature suites across every legacy
industrial workflow.

### What `2.0` must prove

- installation, update, cleanup, and recovery behavior are predictable
- headless SDK and workbench flows share the same conceptual model
- solver claims are benchmark-backed rather than demo-backed
- workflow assets, result reports, optimization metrics, and lineage are
  persistent enough for real engineering review
- agent and mesh execution have clear authority, security, and audit rules
- docs are coherent enough for humans and large-model agents to understand the
  product without oral tradition

The detailed release gate lives in
[commercial-readiness-2.0.md](commercial-readiness-2.0.md).

### What `2.0` should not pretend

- full replacement of incumbent FEM giants
- mature coverage of every industry-specific workflow
- complete third-party operator marketplace maturity
- final rendering and post-processing parity with long-lived commercial tools
- absence of rough edges in advanced deployment scenarios

### The role of the `2.x` line

The `2.x` line should convert the early commercial baseline into a harder
industrial product:

- broaden benchmark coverage and independent verification
- deepen material, contact, nonlinear, multiphysics, and optimization workflows
- mature distributed agent execution and offline/direct mesh operation
- turn operator extension from promising SDK into a real ecosystem
- make project persistence, lineage, and result comparison feel boringly
  reliable

### The `3.0` decision point

`3.0` is the line where Kyuubiki can aim at a direct strategic contest with
established FEM giants.

By `3.0`, the product should have:

- a battle-tested solver and benchmark story
- a serious post-processing and visualization layer
- a strong operator ecosystem and extension path
- enterprise-grade installation, update, security, and audit posture
- distributed execution that is normal product behavior, not a lab demo
- enough real-world project history to justify higher-trust commercial claims

In short:

- `2.0` = credible early commercial trust line
- `2.x` = industrial hardening and ecosystem growth
- `3.0` = direct giant-challenge line

## How to use this page

Use this document when:

- planning the next `1.x` minor
- opening a new `1.x` feature backlog after the `1.20.x` closeout
- deciding whether a change belongs before `2.0`
- checking whether a proposed capability is strengthening the industrial
  baseline or just widening scope

Use these related docs alongside it:

- [current-line.md](current-line.md)
- [tamamono-minor-lines.md](tamamono-minor-lines.md)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
- [accuracy-plan.md](accuracy-plan.md)
- [fem-blender-roadmap.md](fem-blender-roadmap.md)
- [testing-and-ci.md](testing-and-ci.md)
- [installer-remote-control.md](installer-remote-control.md)
