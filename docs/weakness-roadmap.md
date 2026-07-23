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
component attribution through the engine route. One or two optional adaptive
layers now refine nearest changing-response boundaries with normalized,
projectively unique midpoints and a per-layer base-sample budget. Refinement is
now hierarchical: child intervals retain only changing endpoint responses and
halve their parent's projective angle. Probe origin, refinement level, and
parent angle remain visible through the engine route. An independent analytic
boundary at 37% of a 90-degree projective arc now proves eight levels of exact
angle halving while the bracket keeps containing the reference, separating the
refinement convergence claim from any one FE fixture. A ten-element Williams
toggle-frame path now correlates the first external snap-through limit event
against the published analytic load within 5%. A separate 8/16-element pinned
Euler column correlates a sampled bifurcation candidate and two signed,
continued branches against `pi^2 EI / L^2`; the finer switched seed moves
toward the analytic load. An exact repeated pair now recovers the analytic
double eigenvalue and uses orthogonal gauge columns to keep local, same, and
opposite pairwise branches distinct without freezing caller-weighted nonlinear
rotation. A midpoint-coupled pair now externally correlates the symmetric and
antisymmetric eigenvalue split and continues the first connected symmetric
branches. Its antisymmetric invariant path now also resolves the ordered first
and second inertia transitions and continues the secondary opposite-direction
branches against the raised Rayleigh load. A 0.5% right-column stiffness
perturbation now removes those symmetry invariant subspaces: an independent
two-coordinate Rayleigh reduction verifies both mixed eigenloads and
eigenvectors, and the lower and upper mixed single-mode branches continue with
bounded errors. The same case exposed and closed a false-positive boundary:
pairwise branch probes are now solved only inside a degenerate critical
eigenspace. Separated modes return an explicit local rejection instead of four
nominal combinations collapsing onto one physical branch. A three-column
complete midpoint-coupling graph now supplies the complementary positive case:
its uniform mode retains the Euler factor, its two graph-Laplacian modes share
the external `pi^2 EI / L^2 + 6 k L / pi^2` factor, and all eight individual
and pairwise probes in that connected mixed repeated subspace are distinct and
continue with bounded errors. A five-point parameter grid now perturbs the
third-column inertia and one coupling edge independently; all three factors
and the repeated-root split track a closed-form reduced 3-by-3 spectrum. This
completes the linear two-parameter spectral unfolding. Two combined-parameter
points on opposite sides first established external mixed-eigenvector
attribution, signed nonlinear branches, and bounded continuation. A five-point
semicircular path now tracks that identity around the repeated origin by
maximum neighboring mode overlap; every FE critical direction and branch seed
retains the corresponding attribution and bounded continuation. The test was
split into a dedicated triplet submodule before this expansion. Primary
arc-length paths now export a full reusable state contract, validate all DOFs
and constrained components on import, correct the displacement to equilibrium
under changed model parameters, preserve generalized branch orientation, and
return the next reusable state through the engine JSON route. A retained
three-stage regression seeds a selected nonlinear mixed branch on one side,
crosses the exact repeated point, and exits after eigenvalue ordering exchanges
without losing its physical mode identity. This state-seeding capability is
now backed by a typed parameter-path operator. It carries the last accepted
state across compatible models, retains failed attempts, and recursively
inserts interpolated midpoint models under bounded depth and minimum-fraction
controls. The operator is available through Engine and CLI RPC, and a forced
convergence-basin regression proves that failed large jumps recover through
visible quarter-point insertions. Fixed-load state correction now falls back
to a joint displacement/load hyperplane corrector. Optional state-tangent and
target-shape overlaps remain visible; an explicit target shape transports the
predictor only on active shape DOFs and is authoritative over a turning
tangent. A non-singular 3-by-3 serpentine surface now retains external mixed
mode alignment above 0.75 and both numerical error gates below `1e-7`; the
exact repeated point remains covered separately as a subspace crossing. The
lower asymmetric connected branch now also carries a 64-point trajectory from
`1.36e-4 L` to `1.75e-2 L` against an independent complete-elliptic-integral
two-column reduction. Its maximum load error is 2.326%, its minimum mixed-mode
alignment is 0.999989, and every FE point retains both `1e-7` gates. The next
equivalent-topology gate now replaces the direct coupler with two series
members and a free intermediate node, adds an unloaded spectator branch, and
retains a 64-point external trajectory to `1.72e-2 L` with the same 2.33% load
and 0.999989 direction bounds. This covers series/spectator isolation. A
four-column degree-three star now closes the interacting-topology gap: unequal
inertias and three unequal live couplers retain all-column participation over
64 externally seeded points to `1.83e-2 L`, with 2.321% maximum load error,
`4.46e-5` maximum external residual, and 0.999977 neighboring direction
overlap. This work also added a continuation-state identity guard so a
fixed-load Newton correction cannot silently replace a nontrivial imported
branch with the trivial equilibrium. The remaining stability depth is now
material-nonlinear correlation, residual stress, and section interaction.
The first internal
complex-topology isolation reference also proves
single/multi-mode spectral consistency and fixed-load host-response invariance
after adding an unloaded free branch; the 128-mode and 256-inertia caps are
observability limits, not arc-length solver size limits.

## 2.0 Boundary Rule

If a capability cannot be made repeatable, inspectable, and honestly scoped
before `moxi 2.0.0`, it should ship as an experimental or deferred `2.x`
capability rather than weakening the first trust line.
