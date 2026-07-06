# Minimal Industrial Closure

This document defines the smallest honest industrial loop for the `tamamono
1.15.x` to `1.20.x` line.

It is intentionally narrower than the `2.0` commercial-readiness checklist.
The goal here is not to claim broad CAE parity. The goal is to make one bounded
FEM workflow installable, executable, auditable, repeatable, updateable, and
cleanable without hidden UI state or private operational knowledge.

Machine-readable gate data lives in
[minimal-industrial-closure.manifest.json](minimal-industrial-closure.manifest.json).

## Exit Statement

A bounded FEM workflow can be installed, prepared, executed on local or
agent-backed runtimes, audited, repeated, updated, and cleaned without hidden UI
state or private operational knowledge.

## Posture

Current posture: `partial`.

Kyuubiki has crossed the runnable research-prototype line: workflow graphs,
operator families, headless SDK paths, agent descriptors, installer contracts,
reliability gates, and benchmark lanes all exist.

It has not yet crossed the minimum industrial loop line because several pieces
still need to become one repeatable product path:

- executable task IR and operator-package execution must be frozen enough for
  agents
- selected operator families need qualification-grade evidence rather than
  broad smoke coverage alone
- installer-managed remote runtimes must replace ad-hoc lab operations
- run provenance, snapshots, credentials, and update behavior must become
  default visible contracts
- the Workbench must guide users through one obvious main workflow loop

## 1. Executable Task Contract

Minimum state: `partial`.

Required closure:

- workflow graphs lower into language-neutral operator task IR
- TaskIR digests cover executable inputs, config, dataset context, runtime
  hints, and execution program
- agent-facing execution programs do not depend on Elixir, Phoenix, React, or
  private UI state
- external operator packages pass host-side admission and workflow security
  preflight before execution

Next closure work:

- freeze the `1.16` executable task IR compatibility surface
- add golden examples for Rust-native and Elixir-authored task descriptors
- make package fetch, integrity, dispatch, and result serialization visible in
  agent execute mode

Evidence:

- [project-architecture-organization.md](project-architecture-organization.md)
- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [operator-task-ir-digest.md](operator-task-ir-digest.md)
- [workflow-graph.md](workflow-graph.md)
- [workflow-dataset.md](workflow-dataset.md)

## 2. Operator Reliability

Minimum state: `partial`.

Required closure:

- core solve operators have reliability manifest entries
- release gates distinguish runnable, review-level, and qualification-ready
  evidence
- critical operators expose limitations, tolerance policy, and benchmark or
  headless evidence
- workflow output artifacts are validated before becoming runtime state

Next closure work:

- promote selected mechanical, thermal, electromagnetic, and CFD paths to
  qualification candidates
- add convergence diagnostics and failure classification to the critical solve
  path
- retain release evidence bundles for candidate operators

Evidence:

- [operator-reliability.md](operator-reliability.md)
- [physics-coverage-map.md](physics-coverage-map.md)
- [material-research-roadmap.md](material-research-roadmap.md)
- [accuracy-plan.md](accuracy-plan.md)
- [accuracy-baselines.md](accuracy-baselines.md)

## 3. Agent And Orchestration Loop

Minimum state: `partial`.

Required closure:

- Rust agents self-describe execution capability and authority mode
- one agent cannot ambiguously accept multiple orchestrator authorities
- orchestrated and offline-mesh modes are contractually separate
- agents report watchdog-visible failure reasons for runtime failures

Next closure work:

- turn agent execute mode from preflight-centered to package-backed execution
- wire installer-managed remote agents into repeatable lab and local tests
- record scheduler, agent, package, and engine versions in run provenance

Evidence:

- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [agent-control-authority.md](agent-control-authority.md)
- [headless-agent-contract.md](headless-agent-contract.md)
- [operator-library-centralization.md](operator-library-centralization.md)
- [operations.md](operations.md)

## 4. Installer Runtime And Update

Minimum state: `partial`.

Required closure:

- supported platform install paths and managed runtime payloads are visible
- update source, staging, cleanup, and rollback behavior are documented
- remote deployment is installer-owned rather than hidden SSH procedure
- component integrity rules cover required paths and cleanup behavior

Next closure work:

- make self-host runtime bundles the default deployment path for lab nodes
- connect update catalogs to installer apply and rollback flows
- expand component integrity checks from report generation to repair planning

Evidence:

- [installer-remote-control.md](installer-remote-control.md)
- [component-integrity-protocol.html](component-integrity-protocol.html)
- [installation-integrity-contract.html](installation-integrity-contract.html)
- [toolchain-contract.md](toolchain-contract.md)
- [desktop-release-checklist.md](desktop-release-checklist.md)

## 5. Persistence And Provenance

Minimum state: `partial`.

Required closure:

- workflow, input, output, material, operator, runtime, and agent context can be
  traced
- snapshots and result bundles have visible lifecycle and cleanup rules
- headless SDK outputs are machine-readable enough for CI and AI-agent loops
- release evidence avoids local absolute paths

Next closure work:

- standardize immutable run records and project-level result bundle layout
- add replay checks that verify workflow, package, and runtime provenance
- make optimization metrics first-class report fields

Evidence:

- [workflow-dataset.md](workflow-dataset.md)
- [headless-sdks.md](headless-sdks.md)
- [material-research-roadmap.md](material-research-roadmap.md)
- [automated-material-research-example.md](automated-material-research-example.md)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)

## 6. Security And Credentials

Minimum state: `partial`.

Required closure:

- credentials and certificates are not ordinary project files
- remote control behavior is visible and auditable
- operator package and workflow admission fail closed for malformed contracts
- sensitive configuration distinguishes editable values from read-only policy

Next closure work:

- define platform credential-store adapters for desktop and mobile-fronted
  control surfaces
- add signed package and signed update source verification
- promote security audit checks from docs to release gates

Evidence:

- [security.md](security.md)
- [installer-remote-control.md](installer-remote-control.md)
- [component-integrity-protocol.html](component-integrity-protocol.html)
- [operator-sdk.md](operator-sdk.md)

## 7. Workbench User Loop

Minimum state: `partial`.

Required closure:

- users have a clear path from project setup to workflow run, result review,
  report, and snapshot
- Workbench, SDKs, and runtime contracts share the same workflow and result
  concepts
- UI automation anchors are product-owned and stable
- layout and error recovery stay usable at supported resolutions

Next closure work:

- make the main workflow path explicit in the Workbench navigation model
- connect project-scoped operator, workflow-template, and DSL-template
  installation to the same concepts as headless execution
- continue streaming and chunked UI loading for non-hot-path views

Evidence:

- [ui-automation-contract.html](ui-automation-contract.html)
- [frontend-implementation.md](frontend-implementation.md)
- [workflow-graph.md](workflow-graph.md)
- [headless-sdks.md](headless-sdks.md)

## 8. Performance And Regression

Minimum state: `partial`.

Required closure:

- benchmark profiles are repeatable through local and remote lanes
- large graph, workflow catalog, and direct mesh paths have retained summaries
- regression reports explain budget and drift rather than only pass or fail
- compute-heavy tests can move to managed lab nodes without local disk pressure

Next closure work:

- turn 300k and 400k profiles into scheduled remote regression lanes
- add operator-family coverage to benchmark reports
- define minimum release budgets for solver, workflow, SDK, and UI startup
  paths

Evidence:

- [testing-and-ci.md](testing-and-ci.md)
- [operations.md](operations.md)
- [solver-matrix-optimization-pack.md](solver-matrix-optimization-pack.md)
- [material-research-roadmap.md](material-research-roadmap.md)

## Release-Line Use

Use this file as the `1.16` to `1.20` bridge:

- `1.16`
  freeze the executable task IR and operator package boundary
- `1.17`
  close the installer-managed agent/orchestra execution loop
- `1.18`
  make persistence, snapshots, provenance, and credentials default contracts
- `1.19`
  turn benchmark and qualification evidence into repeatable release gates
- `1.20`
  run the bounded industrial workflow as an end-to-end release candidate

The `2.0` commercial-readiness checklist should remain stricter. This document
answers whether the first bounded industrial loop is closed, not whether the
entire product can honestly challenge mature commercial CAE tools.
