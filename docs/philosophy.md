# Product and Engineering Philosophy

Kyuubiki should feel like one system even when it is implemented as several
programs.

That means the browser, orchestrator, solver agents, SDKs, and desktop shells
should share the same values in both behavior and code shape.

## Core Principles

### Engine First

The numerical engine is not a UI detail.

- model contracts should outlive any single frontend surface
- solver/runtime logic should stay reusable outside Phoenix and outside Next.js
- every higher layer should depend on a stable boundary, not internal
  implementation details

### Explicit Boundaries

If two parts can evolve independently, they should be connected by a clear
contract.

- frontend talks to APIs and schemas, not Elixir internals
- orchestrator talks to solver agents through protocol/RPC boundaries
- SDKs target public contracts, not private browser or Phoenix code
- generated/runtime state must stay separate from source-owned files

### Local First, Distributed Ready

The repository should be pleasant on one machine without blocking larger
deployment shapes later.

- local workflows should stay lightweight and host-native
- distributed/cloud paths should reuse the same contracts and semantics
- SQLite defaults are for fast local iteration; PostgreSQL is for shared and
  distributed control planes

### Dense, Calm, Professional UI

Kyuubiki is closer to an engineering workstation than a marketing site.

- interfaces should be compact and information-rich
- panels should reduce clutter through tabs, drawers, and scoped tools
- color should guide attention without becoming visually loud
- motion should clarify state changes, not decorate them

### Progressive Power

Basic tasks should feel obvious. Advanced tasks should be reachable without
turning the entire UI into a cockpit.

- start with the smallest useful surface
- reveal advanced controls when the user enters the relevant mode
- keep immersive/editing workflows separate from high-level study/report flows
- preserve quick paths for repeated expert actions

### Safe by Default

Large models, long jobs, remote agents, and desktop surfaces all need guardrails.

- prefer explicit approvals for risky actions
- expose health, watchdog, and timeout state clearly
- make failure modes inspectable instead of silent
- let users export, persist, and recover work predictably

## Code Philosophy

### Prefer Simple Data Shapes

- JSON-first payloads over framework-specific magic
- explicit structs/types over hidden ad hoc maps when behavior matters
- schema versioning for persisted/imported formats

### Keep Hot Paths Boring

Performance-sensitive code should be predictable before it is clever.

- avoid unnecessary allocations and repeated transformations in solver/data
  paths
- keep rendering work incremental and viewport-aware
- isolate heavy computation from UI orchestration

### Compose Small Domain Modules

- prefer modules that own one responsibility well
- keep domain logic out of view components where possible
- let orchestration code coordinate modules instead of absorbing business logic

### Test the Contract, Not Just the Function

- unit tests cover local behavior
- integration tests cover cross-process flows and boundary assumptions
- smoke tests should validate the real startup path users depend on

### Make Evolution Cheap

- new deployment modes should reuse existing contracts
- new solver capabilities should extend model/schema vocabulary cleanly
- frontends should be replaceable shells over stable platform behavior

## UI Philosophy

### Visual Direction

The visual language should feel like a calm technical instrument panel.

- soft industrial palette
- high legibility first
- restrained accent colors for action and state
- spacious enough to breathe, compact enough for serious work

### Layout Direction

- large work areas should prioritize the viewport/model surface
- secondary information belongs in drawers, tabs, inspectors, and side panels
- controls should sit outside the scene when possible
- scrolling should happen inside bounded panels rather than across the whole app

### Interaction Direction

- direct manipulation where spatial editing matters
- keyboard shortcuts for repeated expert workflows
- reversible operations through undo/redo or explicit transactions
- persistent settings where a preference affects repeated work

## Review Heuristics

When deciding whether a change fits Kyuubiki, ask:

1. Does it strengthen or blur a system boundary?
2. Does it make local development easier without harming distributed shapes?
3. Does it reduce clutter while preserving power?
4. Does it improve reliability and inspectability under failure?
5. Does it make future frontends, agents, or SDKs easier to build?
