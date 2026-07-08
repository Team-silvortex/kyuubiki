# Module Architecture

This document is the high-level module map for the `tamamono 1.15.x` line.
It explains how the current Kyuubiki system is split into product shells,
control-plane services, runtime engines, SDKs, contracts, and verification
gates.

Use this as the first architecture map when deciding where a new capability
belongs.

## Architecture Principle

Kyuubiki should not grow as one large application.

It should grow as a set of cooperating modules connected by stable contracts:

- product shells own user interaction
- control-plane services own orchestration and persistence
- runtime engines own execution
- SDKs own headless access
- schemas and manifests own shared meaning
- checks and tests own drift prevention

The most important rule is that GUI surfaces are clients of runtime
capabilities, not the runtime itself.

## Product Shells

### Hub

Owned paths:

- `apps/hub-gui`
- `apps/desktop-shared`
- `assets/brand`

Responsibilities:

- desktop entrypoint
- workload overview
- runtime posture and health visibility
- docs shelf and operator shell
- navigation into Workbench and Installer

Hub should not own workflow semantics, solver execution, or deployment
internals. It is the operator's system shell.

### Workbench

Owned paths:

- `apps/frontend`
- `apps/workbench-gui`

Responsibilities:

- project workflow UX
- operator graph authoring
- study setup
- result inspection
- browser automation and stable UI surfaces
- WebView/mobile-compatible frontend behavior

Workbench should not own runtime installation, agent topology, or solver
execution implementation. It talks to backend services through thin contracts.

### Installer

Owned paths:

- `apps/installer-gui`
- `deploy`
- release and package layout files under `releases` and `dist`

Responsibilities:

- install, repair, update, and cleanup
- component integrity checks
- remote deployment and host bootstrap
- path policy and storage visibility
- certificate and runtime lifecycle surfaces

Installer is the deployment plane. It should not become a modeling or workflow
authoring surface.

## Control Plane

Owned paths:

- `apps/web`

Responsibilities:

- Phoenix/Plug APIs
- orchestration and job lifecycle
- workflow catalog and graph execution
- persistence and result storage
- operator TaskIR preparation and execution envelopes
- material workflow transforms that run inside the control plane

The control plane is a workload managed by the product shell. It is not the
whole platform. Headless SDKs and agents must be able to interoperate through
protocols without importing frontend code.

## Runtime Data Plane

Owned paths:

- `workers/rust/crates/cli`
- `workers/rust/crates/engine`
- `workers/rust/crates/solver`
- `workers/rust/crates/protocol`
- `workers/rust/crates/installer`
- `workers/rust/templates`

Responsibilities:

- Rust agent process and solver RPC transport
- FEM solver kernels
- workflow execution helpers
- operator TaskIR digest and execution summaries
- agent-native builtins
- installer-native runtime checks and package preflight
- operator SDK template validation

The runtime data plane should execute protocol payloads. It should not know
Workbench component layout, Hub navigation, or Installer screen structure.

## SDK Layer

Owned paths:

- `sdks/rust`
- `sdks/python`
- `sdks/elixir`
- `workers/rust/crates/headless-sdk`

Responsibilities:

- protocol-first access to Kyuubiki capabilities
- headless task preparation and execution
- automation clients for AI and batch workflows
- language-specific convenience wrappers

SDKs are clients of stable contracts. They should not become alternate engines
or duplicate GUI-only assumptions.

## Contract Layer

Owned paths:

- `schemas`
- `docs/*contract*.md`
- `docs/*manifest.json`
- `config/operator-reliability*.json`
- `config/operator-qualification*.json`
- `language-packs`

Responsibilities:

- JSON schemas and examples
- workflow dataset semantics
- operator TaskIR and execution program contracts
- material score contract and manifest
- UI automation contract
- language-pack contract
- operator reliability and qualification evidence

Contract files are the shared language between product shells, control plane,
runtime agents, SDKs, tests, and future model-ingestion tools.

## Verification Layer

Owned paths:

- `make`
- `scripts`
- `tests`
- `apps/*/test`
- `workers/rust/**/tests`
- `evidence`
- `benchmarks`

Responsibilities:

- architecture gates
- line-count and project organization audit
- dependency audit
- version-line audit
- operator reliability checks
- material score contract validation
- UI automation contract checks
- ExUnit and Rust regression suites
- benchmark and qualification evidence capture

The verification layer is part of the architecture. If a contract matters, it
should have a gate in `make architecture-check` or a narrower target that can
be composed into it.

## Data And Control Flow

The normal orchestrated path is:

1. Hub launches or observes a workload.
2. Workbench authors a study, workflow, or operator graph.
3. Workbench calls a backend service contract.
4. The control plane prepares workflow nodes or operator TaskIR.
5. TaskIR carries an execution program and digest.
6. Rust agents verify, preflight, fetch packages, or execute agent-native
   builtins.
7. Results return as contract-shaped summaries and dataset values.
8. Workbench, SDKs, reports, or experiment planners consume the same result
   contract.

Direct-mesh and headless paths skip parts of the GUI/control-plane chain, but
they should not skip protocol identity, digest checks, or result contracts.

## Operator And Workflow Modules

Operator modules should separate:

- catalog metadata
- runtime execution
- TaskIR lowering
- result contract
- tests
- docs and manifests

For example, material candidate scoring now has:

- Elixir runtime implementation
- Rust agent-native implementation
- Elixir tests
- Rust TaskIR tests
- Markdown contract
- machine-readable contract manifest
- architecture-check validator

That pattern should be reused for other operators that become workflow-critical.

## Where New Work Goes

When adding a capability:

- UI interaction goes to Workbench, Hub, or Installer according to product role.
- Control-plane scheduling, persistence, or workflow catalog work goes to
  `apps/web`.
- Agent execution, solver kernels, package preflight, or runtime protocol work
  goes to `workers/rust`.
- Headless access goes to `sdks/*` or `workers/rust/crates/headless-sdk`.
- Shared JSON contracts go to `schemas`.
- Product and architecture source-of-truth prose goes to `docs`.
- Release or validation gates go to `make`, `scripts`, `config`, and tests.

If a change needs to touch several layers, start with the contract and test
evidence before widening the UI.

## Current Architecture Risks

The main risks to keep watching are:

- duplicated runtime semantics between Elixir and Rust
- frontend services growing hidden runtime assumptions
- operator output contracts existing only in tests
- Make targets growing back into the root `Makefile`
- long files crossing the 600-line boundary
- generated artifacts or local machine paths leaking into source control
- headless SDKs lagging behind GUI-only capabilities

The current guards are:

- `make architecture-check`
- `make audit-project-organization`
- `make check-make-modules`
- `make check-material-score-contract`
- `make check-materialization-plan-contract`
- `make check-operator-reliability`
- `make check-ui-automation-contract`
- `make audit-dependencies`

## Related Documents

- [system-overview.md](system-overview.md)
- [app-runtime-boundaries.md](app-runtime-boundaries.md)
- [agent-orchestrator-boundary.md](agent-orchestrator-boundary.md)
- [project-architecture-organization.md](project-architecture-organization.md)
- [repository-structure.md](repository-structure.md)
- [workflow-dataset.md](workflow-dataset.md)
- [material-score-contract.md](material-score-contract.md)
- [minimal-industrial-closure.md](minimal-industrial-closure.md)
