# Weakness Roadmap For Moxi 2.x

This document turns the current weak spots into a concrete roadmap for the
remaining `moxi 2.x` hardening line.

It complements:

- [minimal-industrial-closure.md](minimal-industrial-closure.md)
- [commercial-readiness-2.0.md](commercial-readiness-2.0.md)
- [release-prep-1.9-to-1.20.md](release-prep-1.9-to-1.20.md)

## Roadmap Principle

The goal after `moxi 2.0.0` is not to maximize feature count.

The goal is to make the strongest current capabilities repeatable, explainable,
recoverable, and honest enough for selected early research and industrial
partners.

## Current Tensor Status

The module/function/evidence tensor is now the first navigation gate for this
roadmap. Run `make check-module-function-coverage-tensor` before claiming a
roadmap area is closed.

Current moxi baseline:

- `gap_count`: `0`
- `blocking_gap_count`: `0`
- `thin_evidence_count`: `0`

This means required module/function coordinates are covered by benchmark,
security, and contract evidence. It does not mean the physics and runtime
claims are complete; it means the remaining work should move through concrete
qualification, recovery, fuzz, and user-loop gates instead of architecture
bookkeeping.

## 1. Numerical Trust

Current weak point:

- all release-gated solve operators are now `qualification` level, but several
  qualifications are still scoped around compact retained fixtures
- benchmark-backed accuracy exists across the covered matrix, but the next
  trust jump depends on deeper convergence, perturbation, and reference-tool
  evidence
- some limitations are documented in evidence packets, but they still need to
  become more product-visible in workflow previews and reports

Current moxi hardening focus:

- keep every release-gated physics family at `qualification` without lowering
  the manifest minimum
- turn compact qualification fixtures into richer evidence ladders across
  mechanical, thermal, electromagnetic, CFD/transport, and coupled workflows
- surface explicit failure, assumption, and limitation notes in user-facing
  workflow and export paths

Current progress:

- `solve.solid_tetra_3d` now retains parameter-perturbation and rigid-rotation
  objectivity checks in its active qualification profile; multi-element mesh
  convergence remains an explicit next-depth boundary rather than an implied
  capability
- `solve.thermal_truss_2d` and `solve.thermal_truss_3d` now retain coupled
  thermal-mechanical rigid-rotation checks with free response degrees of
  freedom; arbitrary assemblies and nonlinear thermal mechanics remain outside
  the qualified scope
- `solve.thermal_frame_2d` now retains a thermally graded and mechanically
  loaded rigid-rotation check; `solve.thermal_frame_3d` now has an optional
  explicit `local_y_axis` contract plus arbitrary 3D rigid-rotation evidence,
  while omitted orientation retains the legacy global-reference behavior
- both thermal frame operators now retain manufactured quadratic-field mesh
  convergence across 1, 2, 4, 8, and 16 elements; axial expansion and all
  represented bending directions demonstrate second-order error reduction
- `solve.thermal_frame_3d` now also retains full-response objectivity for a
  non-collinear three-member spatial chain with independent member orientation,
  thermal fields, and terminal mechanical loading
- branched 3D thermal-frame evidence now covers two fully fixed supports, a
  shared three-member junction, redundant thermal restraint, and load
  redistribution under arbitrary rotation
- `solve.thermal_frame_3d` now supports arbitrary-direction translational
  springs with normalized directions, exact `k n n^T` assembly, reported
  displacement/reaction/energy, axial closed-form evidence, and rotated branch
  objectivity
- arbitrary-axis rotational springs now follow the same projector contract on
  rotational degrees of freedom, report rotation/reaction moment/energy, and
  retain torsion closed-form plus rotated branch evidence
- exact arbitrary-direction translation and rotation constraints now use
  orthonormal nullspace elimination rather than penalty stiffness, recover
  reactions from the full residual, reject dependent directions, and retain
  coupled closed-form plus rotated branch evidence
- `solve.buckling_beam_1d` now provides the first geometric-stability slice:
  linear eigenvalue beam-column buckling, critical reference-load factors,
  normalized modes, Euler-column convergence, and dimensional scaling checks
- `solve.buckling_frame_2d` now derives geometric stiffness from a static frame
  preload, retains portal-frame objectivity and beam-formulation cross-checks,
  and resolves sparse repeated modes with oversampled block iteration
- `solve.frame_2d_p_delta` now carries a selected eigenmode imperfection through
  an elastic precritical load path, accepts explicit measured-shape profiles,
  supports linearized P-Delta and incremental corotational kinematics, and
  retains secant-amplification plus multi-member objectivity evidence; its
  experimental spherical arc-length control exposes both force residual and
  path-constraint error, adaptively cuts back oversized radii, continuously
  targets a visible Newton iteration count, and retains a shallow-arch
  limit-point, descending-branch, and segmented member-instability mesh sequence
  with explicit load increments and limit-point events while preserving load
  control as the legacy default
- stability remains screening-only until complex-frame switched-branch
  continuation, automatic problem-scale radius bounds, residual stress,
  material nonlinearity, and independent external correlation exist

Qualification focus:

- add convergence checks beyond the retained closed-form or patch fixtures
- add cross-checks against analytic, literature, manufactured-solution, or
  independent reference cases where practical
- keep retained evidence bundles release-addressable for every promoted
  operator family

Moxi readiness standard:

- Kyuubiki can clearly separate release-qualified, scoped qualification,
  experimental, and deferred solver claims without weakening the mainline
  coverage contract.

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

Current moxi hardening focus:

- keep the operator crate template green with descriptor readiness tests
- expose package readiness in Installer preflight JSON and CI gates
- document the separation between operator SDK and headless SDK everywhere it
  matters

Qualification focus:

- build one complete external-local operator package example
- prove package preflight, package loading, registry binding, run dispatch, and
  failure reporting in one repeatable path
- add operator package compatibility fixtures for future SDK API changes

Moxi readiness standard:

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

Current moxi hardening focus:

- keep agent and orchestra authority modes explicit
- ensure every agent execution failure reports a machine-readable reason
- continue remote-server tests through Installer-owned paths instead of ad-hoc
  SSH operations

Qualification focus:

- add fault-injection tests for agent disconnect, package rejection, runtime
  crash, and scheduler retry
- record scheduler, agent, package, engine, and workflow versions in run
  provenance
- prove centralized and decentralized mesh modes without treating one as a
  second-class fallback

Moxi readiness standard:

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

Current moxi hardening focus:

- keep TaskIR independent of UI, Phoenix, React, and Elixir-only runtime state
- make package fetch, readiness, dispatch, and result serialization visible in
  task previews

Qualification focus:

- freeze the first executable TaskIR compatibility surface
- add golden TaskIR examples for Rust-authored and Elixir-authored tasks
- add digest and replay checks for representative workflows

Moxi readiness standard:

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

Current moxi hardening focus:

- keep GUI actions, headless flows, and Installer preflight aligned around the
  same backend reports
- continue modular UI loading and layout safety work without hiding backend
  state behind UI-only behavior

Qualification focus:

- add one obvious Workbench path: prepare model, choose workflow, preflight,
  run, inspect, export, recover
- make mobile/WebView frontend constraints compatible with remote runtime use

Moxi readiness standard:

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

Current moxi hardening focus:

- keep dynamic library loading behind explicit host policy
- keep credential storage sandboxed and visible
- add more manifest and workflow malformed-input fixtures

Qualification focus:

- fuzz TaskIR, workflow dataset contracts, operator manifests, and package
  preflight parsing
- add red-line tests for path traversal, stale authority, invalid certificates,
  and unexpected runtime residue

Moxi readiness standard:

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

Current moxi hardening focus:

- keep the heat-spreader example reproducible
- expand score contracts and feasibility explanations
- connect headless SDK output, evidence bundles, and report artifacts

Qualification focus:

- add a coupled multiphysics material exploration example
- include parameter sweep, optimization objectives, ranking, failure
  explanations, and exported report artifacts
- run the same example through CLI/headless and Workbench-facing paths

Moxi readiness standard:

- Kyuubiki can show one honest automated materials-research loop that is
  repeatable, inspectable, and useful even if still scoped.

Primary docs:

- [material-research-roadmap.md](material-research-roadmap.md)
- [automated-material-research-example.md](automated-material-research-example.md)
- [material-score-contract.md](material-score-contract.md)

## Priority Order

The recommended order is:

1. deepen numerical trust beyond compact qualification fixtures
2. executable TaskIR stability and replay compatibility
3. operator SDK end-to-end package example
4. agent/orchestra/mesh recovery
5. automated material research flagship
6. security fuzz expansion
7. Workbench main-loop polish and product-visible limitations

Workbench polish matters, but it should not outrun the runtime and numerical
trust foundations.

Current nonlinear-structure progress includes sampled limit-point events and
bounded symmetric-tangent inertia diagnostics. Nonzero inertia changes outside
a limit-point neighborhood now produce explicit bifurcation-candidate brackets.
Transitions up to 128 reduced DOFs now retain a normalized critical mode for
branch construction. Candidate intervals can now be narrowed by configurable,
equilibrium-corrected inertia bisection. Opt-in positive and negative
critical-mode constraints now recover independently equilibrated branch seeds
without mutating the primary path. Both retained seed directions now support
isolated 64-point arc-length continuation with independent radius adaptation,
cutbacks, failure diagnostics, switched-path event fields, rigid-rotation
objectivity, and typed engine JSON transport. An explicit positive finite branch
radius now carries both retained directions through a physical load minimum and
onto the subsequent rising segment without contaminating the primary path. The
optional dimensionless minimum-radius ratio now bounds both adaptive shrinkage
and failed-step cutbacks relative to that visible nominal branch radius. The
same machinery now retains objective positive and negative paths on a branched
arch topology and contains a later non-distinct seed as a local failure. The
transition observer and engine route now also retain one to four ordered modes;
an exact repeated two-mode twin-arch subspace produces four mode-attributed
branch families. An explicit pairwise-combination option additionally probes
the normalized sum and difference of every retained mode pair, records the
component weights and solved projections, and continues all four added
twin-arch directions. A separate caller-weighted vector now selects an
arbitrary direction over two to four retained modes without combinatorial
growth; an exact three-mode repeated subspace retains both signed equilibria,
actual component projections, and continuation. A bounded automatic fan now
adds four deterministic projective directions for three modes and up to sixteen
for four modes, prioritizes full-dimensional combinations, and retains solved
component attribution through the engine route. The next stability milestone
is response-driven adaptive refinement between sampled directions and
independent external complex-topology correlation; the 128-mode and 256-inertia
caps are observability limits, not arc-length solver size limits.

## 2.0 Boundary Rule

If a capability cannot be made repeatable, inspectable, and honestly scoped
before `moxi 2.0.0`, it should ship as an experimental or deferred `2.x`
capability rather than weakening the first trust line.
