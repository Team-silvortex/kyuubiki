# FEM-Blender Roadmap

This document defines the product north star behind Kyuubiki:

`become the Blender of FEM`

That does not mean “look like Blender” in a shallow visual sense.
It means building the same kind of durable creative system for finite-element
engineering that Blender built for digital content creation:

- one primary workbench for day-to-day creation
- one shared data language across tools and extensions
- one node/workflow model for composition
- one automation and scripting surface that is first-class, not secondary
- one plugin/operator ecosystem that can keep growing without collapsing into
  one-off product slices

## What The North Star Really Means

If Kyuubiki reaches this state, users should be able to:

- model, assign materials, set cases, run studies, inspect results, and build
  repeatable workflows inside one coherent workbench
- move between interactive UI work and headless automation without changing the
  conceptual model
- compose built-in and external operators through a stable workflow graph
- rely on workflow-carried data contracts instead of undocumented conventions
- treat automation, operator extension, and batch orchestration as ordinary
  product usage, not expert-only escape hatches

The goal is therefore not “a solver with a UI”.
The goal is:

`a FEM creation platform with a workbench, a workflow language, a contract
layer, a headless SDK surface, and an operator ecosystem`

## Product Translation

To become “FEM Blender”, Kyuubiki needs five layers to mature together.

### 1. Workbench Core

The browser/desktop workbench must become the default creation environment.

It should feel like:

- one place to build and inspect engineering intent
- one place to move from geometry and model state to studies and results
- one place to edit workflows, macros, operators, and reusable assets

Success criteria:

- modeling, materials, workflow, automation, and result review are not split
  into disconnected product islands
- the UI can support both novice direct manipulation and expert dense workflows
- the workbench preserves state, lineage, and reusable assets cleanly

### 2. Workflow And Data Language

This is the deepest non-UI layer of the product.

The workflow graph and workflow dataset contract should do for FEM composition
what node graphs and data blocks do in Blender:

- describe how capabilities connect
- describe what data moves between them
- preserve semantics across operators
- keep composition stable across UI, SDK, and runtime boundaries

Success criteria:

- workflow graphs become the canonical multi-operator composition format
- dataset contracts become the canonical cross-operator value contract
- graph, dataset, port, and artifact semantics stay aligned across frontend and
  SDK implementations

### 3. Headless SDK And Agent Surface

The SDKs are not side tooling.
They are the second main interface to the product.

They should let:

- AI agents
- notebooks
- CI flows
- orchestration services
- remote runtimes

use the same workflow and contract model as the workbench.

Success criteria:

- Python, Rust, and Elixir SDKs expose the same workflow mental model
- submit, validate, and build paths stay aligned across languages
- headless automation can bypass the frontend without becoming a private API
- the SDK surface remains stable enough for long-lived external integrations

### 4. Solver, Result, And Asset Pipeline

A Blender-like product is not just about editing.
It also needs a durable runtime and asset/output pipeline.

For Kyuubiki this means:

- trusted solver execution
- rich result transport and browsing
- artifact persistence and export
- reusable presets, workflow assets, and result bundles
- versioned lineage from input through result

Success criteria:

- result payloads are chunkable, scriptable, and UI-reviewable
- workflow assets and snapshots become reusable long-lived project resources
- runtime execution paths stay compatible with both orchestrated and direct
  headless deployment modes

### 5. Operator And Plugin Ecosystem

Blender became durable because it could keep growing without rewriting itself
for every new domain.

Kyuubiki needs the FEM equivalent:

- built-in operators with strong validation
- external operators with stable descriptors
- shared contracts for ports, schemas, artifacts, and datasets
- a path from “local experiment” to “trusted ecosystem capability”

Success criteria:

- operator integration is contract-first, not UI-first
- external operators can enter the workflow graph without special cases
- validation, docs, and SDK surfaces describe operator behavior consistently

## Current Position

Kyuubiki is already moving in the right direction, but the layers are still at
different maturity levels.

Current strengths:

- the workbench is becoming a real long-lived operator shell
- workflow graph and workflow dataset philosophy now exist explicitly
- headless SDKs now expose workflow submit paths across Python, Rust, and
  Elixir
- workflow validation and builder helpers now exist across all three SDK lines
- frontend automation and headless SDK automation are now being separated into
  cleaner surfaces

Current gaps:

- the workbench is still stronger as an editor than as a full asset-centric
  production environment
- workflow UX is ahead of workflow runtime depth in some areas
- operator extension is defined architecturally but still early as an ecosystem
- history, lineage, asset reuse, and compare tooling need to deepen further
- SDK parity is improving, but long examples and end-to-end shared references
  still need consolidation

## Strategic Principles

To stay on this path, the product should follow a few hard rules.

### Do Not Build One-Off Vertical Features By Default

Every new study family, operator, or automation path should answer:

- does it strengthen the shared workflow model?
- does it fit the shared contract language?
- can it be automated through the SDK surface?
- can it live inside the same workbench instead of spawning a side tool?

If not, it is probably premature.

### Prefer Shared Semantics Over Fast Local Convenience

When choosing between:

- a fast ad hoc payload
- and a slower but reusable contract shape

the long-term product should favor the reusable contract if the cost is
reasonable.

That is how Kyuubiki avoids becoming a pile of unrelated solver screens.

### Treat SDK And Workbench As Peer Interfaces

The UI is not the real product while the SDK is a thin wrapper.
Both are real product surfaces.

The workbench should be the best interactive interface.
The SDK should be the best programmable interface.
Both should speak the same workflow language.

### Grow The Ecosystem Through Contracts, Not Through Privilege

Built-in operators can be richer at first, but they should not depend on hidden
integration privilege forever.

The extension path should move toward:

- declared ports
- declared schemas
- declared dataset values
- declared validation posture

instead of undocumented coupling.

## Execution Stages

### Stage 1. Finish The Shared Language

Primary objective:
make workflow graph, dataset contract, and operator descriptors the stable core
language of composition.

Key outcomes:

- lock graph and dataset semantics harder
- align frontend and SDK examples around the same reference workflows
- improve validation, builder helpers, and sample-backed contract docs

### Stage 2. Turn The Workbench Into The Default FEM Creation Surface

Primary objective:
make the workbench feel like the obvious home for model authoring, study setup,
workflow composition, automation review, and result inspection.

Key outcomes:

- better workflow-authoring ergonomics
- stronger asset and snapshot management
- deeper UI validation, adaptive layout, and production polish
- more reusable project-local workflow assets

### Stage 3. Make Headless And Interactive Paths Symmetric

Primary objective:
make the SDK and workbench two equally serious ways of using the same system.

Key outcomes:

- shared reference workflows across UI and SDKs
- stronger end-to-end automation samples
- runtime paths that stay coherent across desktop, browser, cluster, and CI

### Stage 4. Open The Operator Ecosystem Carefully

Primary objective:
let Kyuubiki grow through operators and plugins without losing coherence.

Key outcomes:

- mature operator SDK
- stable external operator descriptors
- better operator validation and compatibility reporting
- clear promotion path from local operator to reusable ecosystem capability

## Major-Version Commercial Strategy

The FEM-Blender north star should not force `2.0` to carry every final
industrial ambition at once.

### `2.0`: credible early commercial trust line

`2.0` should be the first version that can be offered to selected research
partners, early commercial users, and internal R&D automation teams with a
straight face.

It should prove:

- bounded real engineering workflows can run end to end
- solver claims are backed by benchmarks and documented limits
- workflow assets, material studies, optimization reports, and result lineage
  are durable enough for review
- headless SDK and workbench paths are coherent rather than two separate
  products
- install, update, recovery, security, and audit behavior are predictable

`2.0` is therefore not the final war against incumbent FEM giants.
It is the first trust line where Kyuubiki stops being only a promising platform
and starts being a commercial product.

### `2.x`: industrial hardening and ecosystem growth

The `2.x` line should turn the early commercial baseline into a stronger
industrial platform:

- broader verified solver coverage
- deeper material and multiphysics workflows
- more serious post-processing and comparison tools
- stronger distributed execution and mesh operation
- operator ecosystem growth through stable contracts
- boringly reliable project persistence and lineage

### `3.0`: direct giant-challenge line

`3.0` is the point where Kyuubiki can start positioning itself for direct
strategic competition with established FEM giants.

By then, Kyuubiki should have enough numerical trust, workflow depth,
distributed runtime maturity, operator ecosystem strength, and real project
history to make that claim without sounding aspirational.

## Near-Term Translation For The Current Codebase

In practical terms, the current repository should keep prioritizing:

1. workbench cohesion over isolated feature pages
2. workflow graph and dataset contract hardening over temporary payload hacks
3. SDK parity over language-specific drift
4. operator descriptor maturity over bespoke operator wiring
5. reusable assets, lineage, and workflow history over throwaway execution

That is the shortest path from “promising FEM app” to “FEM creation platform”.

## Related Documents

- [current-line.md](current-line.md)
- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)
- [operator-sdk.md](operator-sdk.md)
- [headless-sdks.md](headless-sdks.md)
- [frontend-style.md](frontend-style.md)
- [frontend-implementation.md](frontend-implementation.md)
