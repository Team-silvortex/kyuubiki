# Kyuubiki

Kyuubiki is a contract-first FEM workstation, workflow system, and distributed
simulation runtime. Its long-term direction is to become a Blender-like
engineering environment for finite-element research: visual when that helps,
headless when automation matters, and open at every protocol boundary.

> The active line is `moxi 2.x`, with `moxi 2.18.3` as the current documented
> development checkpoint. The first planned public release is `daji 3.0.0`.
> Until that gate closes, this repository is an internal hardening line rather
> than a general-availability claim.

## System Shape

Kyuubiki is a set of cooperating products and runtimes, not one monolithic
desktop process.

| Surface | Responsibility |
| --- | --- |
| `Hub` | System entrypoint, workload posture, project launch, and documentation |
| `Workbench` | Modeling, operator workflows, studies, visualization, and result review |
| `Installer` | Install, repair, update, integrity, cleanup, credentials, and remote deployment |
| `Orchestra` | Elixir control plane for scheduling, persistence, results, and coordination |
| `Agent` | Rust data-plane process with one local execution engine instance |
| `Engine + Solver` | Language-neutral task execution, FEM kernels, and operator hosting |

The GUI, control plane, and runtime data plane are deliberately decoupled.
Workbench can use an Orchestra, address an explicit direct-mesh path, or remain
a client of local runtime services. An Agent is either unbound, bound to one
Orchestra, or in an explicit offline peer-mesh mode; it must not accept
simultaneous authority from multiple Orchestras.

Operators are logically centralized at the owning catalog. Agents fetch the
packages required by assigned work instead of carrying a permanent copy of the
entire operator library.

Start with these architecture documents:

- [Current architecture map](docs/current-architecture-map.md)
- [Application and runtime boundaries](docs/app-runtime-boundaries.md)
- [Agent and Orchestra boundary](docs/agent-orchestrator-boundary.md)
- [Agent control authority](docs/agent-control-authority.md)
- [Architecture red lines](docs/architecture-red-lines.md)

## Contract Model

Cross-layer behavior is expressed through versioned contracts rather than GUI
implementation details or language-specific objects.

- `.kyuubiki` is the editable project and workflow bundle.
- `.kcore` is the path-independent frozen research/simulation exchange format.
- TaskIR is the language-neutral executable task representation admitted by
  Agents and Engines.
- Workflow graphs and workflow datasets carry typed values across operators.
- Operator packages expose manifests, capability declarations, integrity data,
  and reliability evidence.
- JSON schemas under `schemas/` are shared by desktop shells, Orchestra,
  Agents, SDKs, stores, and verification tooling.

Read the contract path:

- [Workflow graph](docs/workflow-graph.md)
- [Workflow dataset](docs/workflow-dataset.md)
- [KCore exchange format](docs/kcore-exchange-format.html)
- [Operator TaskIR digest](docs/operator-task-ir-digest.md)
- [Protocols](docs/protocols.md)

## Automation And Extension Surfaces

Kyuubiki keeps three similarly named surfaces separate:

- **Headless SDKs** are Rust, Python, and Elixir clients for controlling the
  same backend capabilities without a GUI.
- **Worker / Operator SDK** is the Rust-only extension surface for implementing
  executable operators that run inside the data plane.
- **Pwdt** is the frontend Python WASM DSL for deterministic automation of the
  fixed, product-owned GUI. It is not the Python Headless SDK.

This separation lets research systems and AI agents build their own control
loops without turning frontend code into runtime authority.

See:

- [Headless SDKs](docs/headless-sdks.md)
- [Operator SDK](docs/operator-sdk.md)
- [Model collaboration SDK](docs/model-collaboration-sdk.html)
- [UI automation contract](docs/ui-automation-contract.html)

## Repository Map

- `apps/`: browser Workbench, Orchestra, and the three Tauri desktop shells
- `workers/`: Rust protocol, Agent, Engine, Solver, Installer, and benchmark crates
- `sdks/`: official Rust, Python, and Elixir Headless SDKs
- `schemas/`: shared workflow, task, material, package, and evidence contracts
- `config/`: architecture topology, coverage tensor, policies, and capabilities
- `deploy/`: portable deployment, update, Agent, and integrity descriptors
- `language-packs/`: local-first product translation packs
- `releases/`: retained release and qualification evidence
- `tests/`: cross-module integration and contract checks
- `docs/`: source-of-truth architecture, operations, and research documentation
- `make/` and `scripts/`: native-first development and verification entrypoints

Generated run output belongs under ignored `tmp/`, `runs/`, `results/`, or
`dist/` paths. Retained claims belong under the versioned `releases/` evidence
surface, not at the repository root.

## Quick Start

Discover the native command surface:

```sh
./scripts/kyuubiki help
make help
```

Run a local development posture:

```sh
make hot-local
```

Other explicit runtime postures are available when their dependencies are
configured:

```sh
make hot-cloud
make hot-distributed
```

Run the lightweight architecture and repository gates:

```sh
make architecture-check
make check-version-line
make check-doc-inventory
make check-doc-book
```

Run focused validation:

```sh
make test-rust
make test-web
make test-frontend
make test-sdk
make test-integration
```

The full verification surface is intentionally larger:

```sh
make verify
```

## Capability Posture

The current tree exposes broad, unevenly mature simulation coverage:

- structural mechanics across springs, trusses, beams, frames, plane elements,
  solids, contact, nonlinear, and cohesive-interface paths
- thermal and thermo-mechanical studies
- electrostatic and magnetostatic studies
- acoustic, modal, transport, simplified flow, and coupled workflows
- material screening and reproducible automated research prototypes
- serial/parallel composite operator flows and distributed execution modes
- retained 500k and 1M benchmark matrices for selected runtime/solver paths

Visibility is not the same as industrial qualification. Exact maturity and
evidence depth are tracked in the module-function-evidence coverage tensor and
the reliability documents:

- [Physics coverage map](docs/physics-coverage-map.md)
- [Operator reliability](docs/operator-reliability.md)
- [Accuracy plan](docs/accuracy-plan.md)
- [Weakness roadmap](docs/weakness-roadmap.md)
- [Testing and CI](docs/testing-and-ci.md)

The remaining `daji 3.0.0` work is dominated by external numerical
correlation, packaged cross-platform GUI/update journeys, deeper distributed
failure testing, and end-to-end usability evidence. Current passing smoke or
benchmark evidence must not be presented as broader certification.

## Documentation

Use the HTML book for the whole-system reading path:

- [Kyuubiki book](docs/book.html)
- [Current moxi line](docs/current-line.md)
- [Documentation index](docs/README.md)
- [Navigation matrix](docs/navigation-matrix.html)
- [Minimal industrial closure](docs/minimal-industrial-closure.md)
- [Commercial readiness](docs/commercial-readiness-2.0.md)

Useful job-oriented entrypoints:

- Research: [Automated material research example](docs/automated-material-research-example.md)
- Operations: [Operations](docs/operations.md)
- Security: [Security](docs/security.md)
- Deployment: [Packaging and deployment](docs/packaging-and-deployment.md)
- Remote nodes: [Installer remote control](docs/installer-remote-control.md)
- Localization: [Language packs](docs/language-packs.md)

## Deployment Safety

Local development defaults are intentionally lightweight, but cloud and
distributed deployments must use explicit configuration and untracked secret
storage. Never commit real database URLs, SSH credentials, tokens, private
keys, certificates, or server-local deployment files.

The Installer owns lifecycle visibility: installation roots, downloads,
immutable component versions, activation, rollback, cleanup, and residue rules
must remain inspectable rather than becoming hidden host state.

## Repository Rules

- Keep source files at or below `800` lines.
- Keep documentation files at or below `2000` lines.
- Prefer native Rust operational tooling over new shell or non-UI Node scripts.
- Keep JavaScript and TypeScript inside UI/runtime-web boundaries.
- Keep GUI, SDK, and runtime semantics aligned through shared contracts.
- Keep generated output out of the repository root and out of Git history.
- Keep all tracked paths repository-relative and all real credentials untracked.

Before handing off a substantial change, run:

```sh
make audit-project-organization
make check-version-line
make check-doc-inventory
make audit-dependencies
git diff --check
```

Kyuubiki is developed by Team Silvortex and distributed under the terms in
[LICENSE](LICENSE).
