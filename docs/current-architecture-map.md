# Current Architecture Map

This is the compact navigation map for the current `daji 3.x` line, reviewed
on October 23, 2026 against the source tree and current constraint-admission
work. It describes ownership and call paths, not installed qualification.
Version declarations belong to the manifests and [current line](current-line.md),
not a second release timeline in this navigation map.

Kyuubiki is a contract-first FEM system, not one monolithic application.
Keep product shells, control clients, runtime services, execution and numerical
kernels separate without adding a new module for every helper.

## Layer Map

```mermaid
flowchart LR
  Hub["Hub: entry and workload routing"]
  UI["Workbench: authoring and visualization"]
  Installer["Installer: deployment and integrity"]
  PWDT["PWDT: fixed UI automation"]
  SDK["Headless SDKs: Rust / Python / Elixir"]
  Control["Orchestra: scheduling and persistence"]
  Agent["Rust Agent: admission, Mesh and receipts"]
  Engine["Engine: execution and operator host"]
  Solver["Solver: numerical algorithms"]
  Worker["Rust Worker / Operator SDK"]
  Lifecycle["Native runtime lifecycle"]

  Hub --> UI
  Hub --> Installer
  PWDT --> UI
  UI --> Control
  UI -->|direct gateway| Agent
  SDK --> Control
  SDK --> Agent
  Control --> Agent
  Agent <-->|explicit offline Mesh| Agent
  Agent --> Engine
  Engine --> Solver
  Worker -->|operator packages| Engine
  Installer --> Lifecycle
  Lifecycle --> Control
  Lifecycle --> Agent
```

Arrows show interactions, not an exhaustive Cargo dependency graph. Shared
schemas, TaskIR, digests, receipts and artifact contracts constrain these paths.
The [module topology](../config/architecture/module-topology.json) groups source
ownership and verification lanes; Cargo manifests define actual crate dependencies.
There are 13 logical topology modules and 15 Rust workspace crates. These are
different views, not missing modules or a requirement to merge executables.

## Product Shells

| Surface | Owns | Does not own |
| --- | --- | --- |
| Hub | Entry, project routing, runtime posture, documentation | Workflow editing or numerical execution |
| Workbench | Modeling, operator graphs, research workflow, visualization | Runtime installation or solver internals |
| Installer | Deployment, repair, updates, credentials and component integrity | Engineering project authoring |
| Shared desktop UI | Shared assets, bridge helpers and stable automation anchors | A fourth shell or runtime service |

The three native desktop shells remain independent applications. Share native
lifecycle logic and UI assets instead of merging their user-facing roles.
Mobile/WebView clients use remote capabilities and do not imply local Agent or
Orchestra deployment. See [app-runtime-boundaries.md](app-runtime-boundaries.md).

## Control Plane

`apps/web` owns Orchestra's scheduling, API, job persistence, workflow execution,
catalog lowering and recovery coordination. Orchestra is a managed workload,
not the whole product. Single-Agent direct use and offline Mesh do not require
routing every operation through Orchestra.

The central website/store is a named self-host service surface within the
control-plane topology, not a second platform or an additional top-level module.
Keep its service policy and deployment boundaries explicit; its existence does
not qualify native Agent authentication. See [module-architecture.md](module-architecture.md).

## Runtime Data Plane

| Rust crate | Responsibility |
| --- | --- |
| `protocol` | RPC/TaskIR contracts, digests and pure shared admission rules |
| `cli` | Agent process, transport, authority, Mesh, durable receipts and native command entrypoints |
| `engine` | Reusable execution dispatch, workflow helpers and operator host |
| `solver` | Numerical kernels, accuracy gates and numerical recovery strategies |
| `operator-sdk` | Rust-only executable operator extension and ABI contracts |
| `desktop-runtime` | Native local-service lifecycle, process ownership and runtime layout |
| `installer` | Install/update/repair, credentials, package and deployment validation |
| `platform` | Shared OS/platform primitives |
| `headless-sdk` | Rust UI-free control and research client |
| `project-automation` | Stored project automation to Headless plans and execution reports |
| `project-bundle` | Editable `.kyuubiki` project container operations |
| `kcore` | Frozen `.kcore` computation exchange |
| `deploy-server` | Native artifact delivery service |
| `benchmark` | Native solver/runtime performance lanes |
| `script-runner` | Repository operations, checks and native maintenance tools |

All crate paths above are under `workers/rust/crates/`. Independent crate
boundaries do not imply independent running processes. Engine and Solver are
linked libraries used by the Agent; there is no need for a solver microservice
just to preserve ownership. The same applies to shared policy helpers in Protocol.
Use [Cargo.toml](../workers/rust/Cargo.toml), not a prose count, when building.

## Execution And Recovery Path

1. GUI, PWDT-driven UI or a Headless client creates a request. Elixir/Rust task
   descriptions lower into language-neutral TaskIR, not Elixir code on the Agent.
2. The selected control path submits to Orchestra or directly to an Agent.
3. Agent validates digest, admission, capability, ownership and input contract.
4. An admitted built-in uses its native adapter; admitted solver TaskIR dispatches
   through Engine. External packages require resolution, integrity checks and
   an attached operator runtime. `execute` exists; it is not only a reserved mode.
5. Results and receipts retain execution identity and provenance. Recovery reads
   the exact original evidence; unknown outcomes never imply safe replay or
   migration to another peer.

TaskIR solver capability is narrower than the complete direct-RPC solver catalog.
The current native solver adapter admits `solve.bar_1d`, `solve.modal_frame_2d`
and `solve.modal_frame_3d`. Other native built-ins and external packages have
their own gates. Neither an operator name nor a successful preflight proves a
completed solve or production qualification. Source anchors:

- [TaskIR dispatch](../workers/rust/crates/cli/src/operator_task_runtime.rs)
- [Engine solver adapter](../workers/rust/crates/cli/src/operator_task_runtime/engine_solver.rs)
- [Runtime and recovery book](book-ch04-runtime-modes.html)

## Numerical Admission Ownership

Numerical acceptance belongs in Solver, after reconstruction and before
successful result publication. Engine dispatch/workflows and Agent/SDK clients
must not grow their own residual policy, relaxed tolerance or hidden retry.
Transport admission, TaskIR capability and numerical acceptance are different
contracts; implementing a kernel gate does not widen the TaskIR allowlist.

| Equation family | Solver owner | Admission boundary |
| --- | --- | --- |
| Coordinate zero/prescribed constraints | [Shared sparse validator](../workers/rust/crates/solver/src/linear_constraint_validation.rs) and [chain validator](../workers/rust/crates/solver/src/chain_tridiagonal_constraint_validation.rs) | Complete unknowns against pre-elimination equations; fixed values are exact data and support reactions may be nonzero |
| Directional thermal-frame supports | [Projected constraint validator](../workers/rust/crates/solver/src/thermal_frame_3d_constraint_validation.rs) | Support identities and free-direction equilibrium in restored physical coordinates |
| Transient spring Newmark steps | [Transient operator](../workers/rust/crates/solver/src/transient_spring_1d.rs) using the shared sparse validator | Full velocity increment against the original effective system before physical state/history updates |
| Nonlinear spring/contact Newton steps | [Shared nonlinear path](../workers/rust/crates/solver/src/nonlinear_spring_1d.rs) using the shared sparse validator | Full unscaled correction against the current tangent/residual before line search; every evaluated state also checks complete dimensions and fixed identity |
| Harmonic spring complex equations | [Complex original-equation validator](../workers/rust/crates/solver/src/harmonic_spring_constraint_validation.rs) and [frequency-product range helpers](../workers/rust/crates/solver/src/harmonic_spring_frequency_range.rs) | Each frequency checks complete complex state, exact support identities, the reduction map and original element/mass/load terms; dynamic coefficients and response products defer exponent restoration until all factors are combined |

These are explicitly wired consumer scopes, not automatic coverage of all
operators. A nonlinear state is not a static tangent solution; a dynamic
increment is not a static physical displacement. Family-specific reaction and
postprocessing checks remain separate from the shared equation gate.

Harmonic [complex arithmetic](../workers/rust/crates/solver/src/harmonic_spring_complex_range.rs)
also owns exceptional elimination updates, complete-row substitution replay and
element-force product restoration. A wide temporary numerator may be divided
before restoration; an unrepresentable final coefficient, force or extension
must still fail instead of becoming a successful zero-valued result.

Solver unit tests own mathematical and range counterexamples. Engine integration
tests own serialized dispatch/workflow equality, gate reachability and failure
propagation. [Validation profiles](../config/operator-validation-profiles/linear-constraints.json)
register commands; [tensor evidence](../config/architecture/module-function-coverage-evidence/runtime-constraint-validation.json)
binds their bounded claims. Cooperative cancellation and same-input replay are
not durable process-restart or host-power-loss evidence.

## SDK And Extension Surfaces

- Headless SDKs are Rust/Python/Elixir control clients. The official CLI is a
  consumer, not a mandatory gateway for users writing their own applications.
- Worker / Operator SDK is Rust-only and extends executable operators. It does
  not install arbitrary frontend UI or replace the Headless SDK.
- PWDT is the frontend Python WASM automation surface. Workbench hosts the full
  Pyodide console; Hub's launcher/stub and Installer's planned restricted
  diagnostics are not equivalent execution hosts.

See [headless-sdks.md](headless-sdks.md), [operator-sdk.md](operator-sdk.md) and
[the book reading paths](book-ch08-reading-paths.html).

## Authority Rules

The intended modes remain one owning Orchestra, standalone direct control, or
explicit offline Mesh. Centralized and decentralized paths share runtime
contracts; neither is a UI-dependent fallback. Operator packages are fetched
from their configured authority as needed rather than replicated in full to
every Agent.

Keep desired authority rules separate from enforced transport security. The
native listener currently defaults to loopback and admits configured source
IPs, but this is not TLS or cryptographic peer/controller authentication.
Agent, Installer and runtime preflight share the policy parser. Managed restart
validates that policy before stopping existing services; status distinguishes
configuration from the policy observed on a running process. See the
[bounded containment and preflight evidence](../reports/agent-network-access-20261016.md).

## Verification Spine

- [Topology](../config/architecture/module-topology.json) owns logical modules,
  paths, relationships and verification lanes.
- [Matrix](../config/architecture/module-function-coverage-matrix.json) owns
  module/function participation.
- [Tensor](../config/architecture/module-function-coverage-tensor.json) owns
  required evidence dimensions, named scenario scopes and release targets.
- [Module architecture](module-architecture.md) explains the scoring rules;
  [current line](current-line.md) and dated `reports/` own progress and experiments.

The October 16 calibration remains the dated baseline. Run
`make check-module-function-coverage-tensor` for the working-tree result rather
than copying a growing sequence of historical counts into this map. A successful
structural check does not imply release readiness. Local tests, same-host TCP,
installed journeys, multi-host partitions and host power loss are distinct scopes.

Solver experiment results remain in their original reports and current-line
index. Removing duplicate experiment narratives from this overview does not
remove tests, failures, qualification requirements or numerical acceptance gates.

## Repository And Disk Ownership

Source boundaries and cache directories solve different problems. Do not merge
Agent, Engine, SDK and desktop shells merely to reduce build artifacts.

- Versioned source, schemas, fixtures, lockfiles and retained evidence are kept.
- Incremental caches and hashed test executables are regenerable. Use the native
  `make disk-usage` preview and `make clean-dev-cache` cleanup while builds/tests
  are idle; dependency libraries, top-level tools and release bundles are retained.
- `tmp/` is mixed: reports can be temporary, but journals, SQLite files and
  execution receipts are state. Ignored by Git does not mean safe to delete.
- Installed Apps, runtime versions, credentials and server storage are separate
  scopes managed through Installer, not blanket repository cleanup.

See [project-architecture-organization.md](project-architecture-organization.md)
for the storage policy and [repository-structure.md](repository-structure.md)
for paths. This pass does not change runtime behavior, merge packages or relabel
existing evidence.
