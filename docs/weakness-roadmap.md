# Weakness Roadmap To Moxi 2.0

This document turns the current weak spots into a concrete roadmap for the
remaining `tamamono 1.x` hardening line.

It complements:

- [minimal-industrial-closure.md](minimal-industrial-closure.md)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
- [release-prep-1.9-to-1.20.md](release-prep-1.9-to-1.20.md)

## Roadmap Principle

The goal before `moxi 2.0.0` is not to maximize feature count.

The goal is to make the strongest current capabilities repeatable, explainable,
recoverable, and honest enough for selected early research and industrial
partners.

## 1. Numerical Trust

Current weak point:

- many operators are runnable and composable, but not yet qualification-grade
- smoke tests are broader than benchmark-backed accuracy evidence
- some limitations are implicit instead of product-visible

`1.18.x` focus:

- keep every broad physics family at least smoke-covered
- identify the first qualification candidates across mechanical, thermal,
  electromagnetic, CFD/transport, and coupled workflows
- add explicit failure and limitation notes to weak solver families

`1.19.x -> 1.20.x` focus:

- add convergence checks for selected families
- add cross-checks against analytic, literature, or independent reference
  cases where practical
- retain evidence bundles for candidate operators

`2.0` exit standard:

- Kyuubiki can clearly separate verified, review-level, partial, and
  experimental solver claims.

Primary docs:

- [accuracy-plan.md](accuracy-plan.md)
- [accuracy-baselines.md](accuracy-baselines.md)
- [operator-reliability.md](operator-reliability.md)

## 2. Rust Operator SDK Industrialization

Current weak point:

- the Rust-only operator SDK has descriptors, manifests, readiness checks, and
  preflight, but the third-party author journey is still young
- external packages need stronger end-to-end examples from authoring to
  package admission and execution

`1.18.x` focus:

- keep the operator crate template green with descriptor readiness tests
- expose package readiness in Installer preflight JSON and CI gates
- document the separation between operator SDK and headless SDK everywhere it
  matters

`1.19.x -> 1.20.x` focus:

- build one complete external-local operator package example
- prove package preflight, package loading, registry binding, run dispatch, and
  failure reporting in one repeatable path
- add operator package compatibility fixtures for future SDK API changes

`2.0` exit standard:

- a competent Rust developer can write, package, preflight, and run a custom
  operator without private project knowledge.

Primary docs:

- [operator-sdk.md](operator-sdk.md)
- [operator-library-centralization.md](operator-library-centralization.md)

## 3. Agent, Orchestra, And Mesh Reliability

Current weak point:

- authority boundaries are documented, but long-running failure behavior still
  needs more evidence
- distributed execution must prove recovery from partial failure, package
  fetch failure, node loss, and stale authority state

`1.18.x` focus:

- keep agent and orchestra authority modes explicit
- ensure every agent execution failure reports a machine-readable reason
- continue remote-server tests through Installer-owned paths instead of ad-hoc
  SSH operations

`1.19.x -> 1.20.x` focus:

- add fault-injection tests for agent disconnect, package rejection, runtime
  crash, and scheduler retry
- record scheduler, agent, package, engine, and workflow versions in run
  provenance
- prove centralized and decentralized mesh modes without treating one as a
  second-class fallback

`2.0` exit standard:

- one bounded workflow can survive ordinary distributed-system failures without
  cascading into an unexplained global failure.

Primary docs:

- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [headless-agent-contract.md](headless-agent-contract.md)
- [installer-remote-control.md](installer-remote-control.md)

## 4. Executable Task IR Stability

Current weak point:

- Elixir can remain the fast authoring layer, but the executable structure that
  reaches agent engines must be language-neutral
- the TaskIR surface still needs more golden examples and compatibility gates

`1.18.x` focus:

- keep TaskIR independent of UI, Phoenix, React, and Elixir-only runtime state
- make package fetch, readiness, dispatch, and result serialization visible in
  task previews

`1.19.x -> 1.20.x` focus:

- freeze the first executable TaskIR compatibility surface
- add golden TaskIR examples for Rust-authored and Elixir-authored tasks
- add digest and replay checks for representative workflows

`2.0` exit standard:

- agent engines execute a stable task representation, not a private frontend or
  language-runtime convention.

Primary docs:

- [operator-task-ir-digest.md](operator-task-ir-digest.md)
- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)

## 5. Frontend And Runtime Consistency

Current weak point:

- the architecture says GUI, headless SDKs, agent, and orchestra should share
  backend capabilities, but experience parity is not fully proven
- Workbench still needs an obvious main workflow loop for serious users

`1.18.x` focus:

- keep GUI actions, headless flows, and Installer preflight aligned around the
  same backend reports
- continue modular UI loading and layout safety work without hiding backend
  state behind UI-only behavior

`1.19.x -> 1.20.x` focus:

- add one obvious Workbench path: prepare model, choose workflow, preflight,
  run, inspect, export, recover
- make mobile/WebView frontend constraints compatible with remote runtime use

`2.0` exit standard:

- the GUI is a first-class client of the same system, not a special runtime
  that secretly owns core behavior.

Primary docs:

- [app-runtime-boundaries.md](app-runtime-boundaries.md)
- [ui-architecture-migration.md](ui-architecture-migration.md)
- [mobile-gui-runtime-boundary.md](mobile-gui-runtime-boundary.md)

## 6. Security And Fuzz Coverage

Current weak point:

- security checks exist, but fuzz and hostile-input coverage should become more
  systematic around manifests, TaskIR, workflow datasets, credentials, and
  package loading

`1.18.x` focus:

- keep dynamic library loading behind explicit host policy
- keep credential storage sandboxed and visible
- add more manifest and workflow malformed-input fixtures

`1.19.x -> 1.20.x` focus:

- fuzz TaskIR, workflow dataset contracts, operator manifests, and package
  preflight parsing
- add red-line tests for path traversal, stale authority, invalid certificates,
  and unexpected runtime residue

`2.0` exit standard:

- common malformed or hostile inputs fail closed with useful diagnostics and no
  hidden residue burden.

Primary docs:

- [security.md](security.md)
- [architecture-red-lines.md](architecture-red-lines.md)
- [packaging-and-deployment.md](packaging-and-deployment.md)

## 7. Automated Material Research Loop

Current weak point:

- the research loop is real enough to be promising, but it still needs one
  flagship repeatable example that explains why Kyuubiki is different
- optimization metrics and reports need to feel like product primitives, not
  demo notes

`1.18.x` focus:

- keep the heat-spreader example reproducible
- expand score contracts and feasibility explanations
- connect headless SDK output, evidence bundles, and report artifacts

`1.19.x -> 1.20.x` focus:

- add a coupled multiphysics material exploration example
- include parameter sweep, optimization objectives, ranking, failure
  explanations, and exported report artifacts
- run the same example through CLI/headless and Workbench-facing paths

`2.0` exit standard:

- Kyuubiki can show one honest automated materials-research loop that is
  repeatable, inspectable, and useful even if still scoped.

Primary docs:

- [material-research-roadmap.md](material-research-roadmap.md)
- [automated-material-research-example.md](automated-material-research-example.md)
- [material-score-contract.md](material-score-contract.md)

## Priority Order

The recommended order is:

1. numerical trust
2. executable TaskIR stability
3. operator SDK end-to-end package example
4. agent/orchestra/mesh recovery
5. automated material research flagship
6. security fuzz expansion
7. Workbench main-loop polish

Workbench polish matters, but it should not outrun the runtime and numerical
trust foundations.

## 2.0 Boundary Rule

If a capability cannot be made repeatable, inspectable, and honestly scoped
before `moxi 2.0.0`, it should ship as an experimental or deferred `2.x`
capability rather than weakening the first trust line.
