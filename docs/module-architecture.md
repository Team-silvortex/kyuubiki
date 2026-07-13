# Module Architecture

This document is the high-level module map for the `tamamono 1.19.x` line.
It explains how the current Kyuubiki system is split into product shells,
control-plane services, runtime engines, SDKs, contracts, and verification
gates.

Use this as the first architecture map when deciding where a new capability
belongs.

The strict, machine-readable source of truth is
`config/architecture/module-topology.json`. This document explains that
topology; the topology file drives benchmark and security-test grouping.
The companion coverage matrix is
`config/architecture/module-function-coverage-matrix.json`; it checks whether
each module is covered across the main function paradigms.
The coverage tensor is
`config/architecture/module-function-coverage-tensor.json`; it combines the
module topology, function matrix, benchmark lanes, and security lanes into a
three-axis gap map.

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

## Strict Module Topology

The topology contract is intentionally stricter than prose architecture notes.
Every top-level module must declare:

- `id`: stable module identifier used by reports and gates
- `layer`: one of `product_shell`, `control_plane`, `runtime_data_plane`,
  `sdk`, `contract`, or `verification`
- `owned_paths`: repository-relative paths owned by the module
- `depends_on`: module IDs that must stay contractually upstream
- `benchmark_lanes`: performance lanes that should include this module
- `security_lanes`: security lanes that should include this module
- `risk_tags`: short labels for targeted review and future dashboards

The same topology file also carries `lane_test_plan`. That plan maps each
benchmark and security lane to suggested commands with a scope:

- `local`: safe local gate
- `integration`: cross-process or service-level test
- `benchmark`: performance comparison or profile run
- `remote`: lab/self-hosted runner path
- `release`: broad release-style gate

The checker in `scripts/check-module-topology.mjs` enforces:

- the topology schema version
- all owned paths exist and are repository-relative
- owned paths are not duplicated between modules
- every dependency points at a declared module
- the dependency graph is acyclic
- all benchmark and security lanes are declared centrally
- every lane has at least one suggested test command
- all required architecture layers are represented

This gives us a stable topology spine for later automation. A benchmark runner
can select modules and commands by `benchmark_lanes`; a security audit can
select modules and commands by `security_lanes`; a release report can show
risk coverage by `risk_tags`.

Run `make build-module-topology-report` to write the consumed view to
`tmp/module-topology/`. The generated report includes:

- `index.json`: machine-readable module, benchmark-lane, and security-lane
  index
- `README.md`: human-readable lane tables for review
- `index.html`: lightweight browser view for release/nightly artifacts

## Module Function Matrix

The module topology answers "what owns what". The module function matrix
answers "which module participates in which functional paradigm". This is the
coverage view for avoiding silent gaps across:

- product surfaces
- runtime APIs
- solver execution
- workflow composition
- validation
- benchmark
- security
- persistence/provenance
- deployment/update
- headless SDK access

Run `make check-module-function-matrix` to validate the matrix and write
`tmp/module-function-matrix-report.json` plus a Markdown table. Required
module/paradigm cells cannot be missing, and all rows must match declared
module topology IDs.

## Module Function Coverage Tensor

The matrix is two-dimensional. The coverage tensor adds a third dimension:
evidence depth.

Its axes are:

- `module`: the module ID from `config/architecture/module-topology.json`
- `function_paradigm`: the paradigm from
  `config/architecture/module-function-coverage-matrix.json`
- `evidence_depth`: required state, matrix status, benchmark evidence,
  security evidence, and derived gap level

Run `make check-module-function-coverage-tensor` to generate
`tmp/module-function-coverage-tensor.json` and
`tmp/module-function-coverage-tensor.md`.

The tensor is the review map for "where have we not really done it yet". It
does not replace detailed tests. It points each weak coordinate at the relevant
benchmark and security lanes so the next task can pick a concrete module,
function paradigm, and evidence path.

Run `make check-contracts-runtime-api-surface` when shared contracts gain or
move runtime API sources. It validates
`config/architecture/contracts-runtime-api-surface.json`, including the
frontend, protocol, orchestra, and central-store contract families that other
modules consume. The central-store family includes the central database table
contract plus read-only database policy/status endpoints, so deployment checks
can tell whether a build carries the expected persistence surface before any
write-side publishing is enabled. Run `make check-central-store-contract` when
changing the future center-server catalog, login/session policy, language-pack
distribution, database policy/status, or matching frontend client surface.

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
- `apps/web/lib/kyuubiki_web/central_store.ex`
- `apps/web/lib/kyuubiki_web/central_store_router.ex`

Responsibilities:

- JSON schemas and examples
- workflow dataset semantics
- operator TaskIR and execution program contracts
- material score contract and manifest
- UI automation contract
- language-pack contract
- central-store catalog, session-policy, database-policy, and database-status contract
- central-server JSON schemas
- central readiness report schema and retained evidence check
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
- central readiness report generation and retained report validation
- ExUnit and Rust regression suites
- benchmark and qualification evidence capture

The verification layer is part of the architecture. If a contract matters, it
should have a gate in `make architecture-check` or a narrower target that can
be composed into it.

## Benchmark And Security Lanes

The current benchmark lanes are:

- `ui_startup`: WebView, layout, workflow UI, and desktop shell startup/render
  cost
- `workflow_catalog`: workflow catalog search, topology, package, and graph
  authoring cost
- `control_plane`: Phoenix API, orchestration, persistence, and workflow
  catalog service cost
- `runtime_solver`: Rust engine, protocol, solver kernels, TaskIR, and agent
  execution cost
- `mesh`: direct mesh, distributed agent, heartbeat, routing, and large-node
  fanout cost
- `sdk_headless`: Rust/Python/Elixir headless SDK examples, batch workflows,
  and research automation
- `installer_release`: install, update, repair, remote deployment, and disk
  hygiene cost

The current security lanes are:

- `ui_boundary`: GUI/runtime decoupling, automation selectors, local storage,
  and WebView trust
- `api_auth`: control-plane, direct-mesh, token, cluster identity, replay, and
  export authorization
- `runtime_sandbox`: TaskIR, workflow JSON budgets, operator package admission,
  and solver execution guards
- `supply_chain`: dependency audit, update catalog, package integrity, and
  release provenance
- `credential_storage`: credential vault, in-memory secrets, mobile-compatible
  boundaries, and SSH material
- `remote_deploy`: remote installer, SSH deployment, host trust, cleanup, and
  residual policy
- `data_contract`: schemas, workflow datasets, result contracts, language
  packs, and import/export validation

When a new benchmark or security gate is added, it should either map to one of
these lanes or add a lane to `config/architecture/module-topology.json` with a
clear reason.

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
- `make check-module-topology`
- `make check-module-function-matrix`
- `make build-module-topology-report`
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
