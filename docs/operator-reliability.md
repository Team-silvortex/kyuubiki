# Operator Reliability

Kyuubiki now keeps operator reliability as a machine-readable contract instead
of a loose project memory item.

The source of truth is:

- `config/operator-reliability-manifest.json`
- `config/operator-reliability/*.json`
- `config/operator-validation-profiles.json`
- `config/operator-qualification-roadmap.json`
- `config/operator-qualification-evidence-kits.json`
- `schemas/operator-reliability-manifest.schema.json`
- `schemas/operator-reliability-shard.schema.json`
- `schemas/operator-qualification-roadmap.schema.json`
- `schemas/operator-qualification-evidence-kits.schema.json`
- `make check-operator-reliability`
- `make check-operator-validation`
- `make verify-operator-validation`

The manifest index maps release-level metadata to per-domain shards. Each shard
maps its `physics-coverage` solve operators to:

- its benchmark template
- its physics domain
- its current trust level
- its headless workflow evidence
- its test or accuracy-baseline evidence
- its explicit limitations

`config/operator-validation-profiles.json` is the first Operator Validation
Harness contract. It groups operators into validation profiles and records:

- analytic or cross-check methods
- local formal invariants
- evidence paths that must remain repo-relative
- commands that can execute the validation profile

The profile contract supports optional repo-relative `profile_shards`. The
native validation runner and qualification-readiness builder merge those shard
profiles with the main file before checking duplicate profile ids, evidence
paths, and command allowlists. New qualification profiles should prefer shards
when the main file is close to the source-size ceiling.

Each validation profile also declares its qualification mapping:

- `profile_role=release_candidate` means the profile is itself the candidate
  expected by release evidence and qualification records.
- `profile_role=component_profile` means the profile is a narrower validation
  lane that feeds a broader `qualification_candidate_id`.

This keeps the status tensor from treating focused electrostatic, heat, or CFD
screening lanes as missing release candidates when they are intentionally
rolled into broader qualification kits.

The input profile shape is retained as
`schemas/operator-validation-profiles.schema.json`.

`make check-operator-validation` validates the profile contract and writes
`tmp/operator-validation-report.json` without running heavy commands.
`make verify-operator-validation` executes the declared commands and writes the
same report with command status and output tails. Both targets now use the
native `kyuubiki-script-runner check-operator-validation` path. This is not a whole-system
formal proof; it is a practical lane for accumulating executable local
invariants and cross-validation evidence per operator family.
The report shape is retained as
`schemas/operator-validation-report.schema.json` with
`schemas/examples.operator-validation-report.json` as the fixture.

Validation commands use a small controlled kind vocabulary:

- `analytic`: closed-form or derived reference checks
- `cross_check`: independent implementation, shape, or representation checks
- `boundary_regression`: boundary-condition and edge-case regression checks
- `invariant`: local conservation, finiteness, or sign invariants
- `contract`: repository-level contract checks that support the profile

The first validation profiles cover:

- `line-field-closed-form`: 1D analytic closed-form checks and tolerance policy
- `stokes-flow-screening`: CFD Stokes screening boundary and tolerance checks
- `screening-cfd-boundary`: release-candidate aggregation for the Stokes
  screening boundary evidence kit
- `electrostatic-plane-patch`: triangle/quad constant-gradient electric field
  and stored-energy patch checks
- `heat-plane-patch`: triangle/quad temperature-gradient and heat-flux patch
  checks

For `moxi 2.0.x`, the manifest also declares
`minimum_coverage_level: review`. `make check-operator-reliability` treats this
as a release gate, so future edits cannot silently downgrade a covered operator
back to `baseline` or `smoke`. The Make target runs the checker self-test first
to keep the trust-level ordering and release-gate behavior from regressing.

The shard layout keeps each domain contract below the project source-size limit
while preserving one release-level verification command.

The Rust engine now applies a workflow security preflight before executing a
graph. The guard rejects unsupported workflow schema versions, excessive graph
sizes, duplicate node or edge ids, malformed identifiers, unsupported
operators, invalid edge references, and input artifacts that target non-input
nodes. It also applies JSON security budgets to workflow node configs, workflow
dataset contracts, input artifacts, and output artifacts. This keeps GUI,
headless SDK, and agent/orchestra execution paths behind the same first safety
gate.

Workflow execution is still fail-fast by default. A non-core node can opt into
node-level recovery with `config.on_error: "skip"` or
`config.recovery.on_error: "skip"`; `fail` is also accepted as an explicit
fail-fast policy. Other recovery policy values are rejected during workflow
security preflight. When a recoverable node returns an error or panics, the run
records the node as `failed`, preserves its error message in `node_runs`, rolls
back artifacts written by the failed node, skips downstream nodes that cannot
resolve artifacts, and continues independent branches. This prevents
recoverable analysis/reporting failures from cascading across the whole graph
without hiding the failure from SDK or GUI callers.

The qualification roadmap lives at
`config/operator-qualification-roadmap.json`. The same checker validates that
roadmap candidates reference existing manifest operators and already satisfy
the roadmap's minimum candidate level.
Each roadmap candidate also carries a machine-readable qualification posture:

- `target_level`
  the trust level the candidate is trying to reach, usually `qualification`
  but intentionally `review` for screening-only families that should not be
  over-promoted
- `evidence_phase`
  whether evidence is still `planned`, actively `collecting`,
  `ready_for_review`, or `blocked`
- `primary_blocker`
  the single most important reason the candidate cannot be promoted yet
- `preferred_validation_lane`
  the make target release owners should run first when refreshing evidence
- `release_gate_impact`
  whether the candidate is a release blocker, release watch item, or
  experimental-only constraint

These fields make the weak point explicit: Kyuubiki should advance numerical
trust through retained evidence, not by silently changing a coverage label.

The qualification evidence kits live at
`config/operator-qualification-evidence-kits.json`. They are deliberately
planning-grade: they list the artifacts that must be collected before a
roadmap candidate can be promoted into real `evidence.qualification` manifest
entries. The checker keeps every kit tied to an existing roadmap candidate and
prevents operators from drifting into the wrong qualification group.
Command-backed artifacts may also declare an `artifact_check_command`, so the
readiness report can show both the evidence capture step and the acceptance
step for a generated release bundle.
`make build-operator-qualification-readiness` writes a local JSON report that
summarizes which roadmap artifacts are present, command-backed, missing, or not
started. The generated report also includes a `next_actions` queue so release
owners can see the highest-priority evidence collection step without manually
diffing every candidate kit. Its summary also groups candidates by target
trust level, evidence phase, and release-gate impact, so CI and future UI
surfaces can distinguish release blockers from watch items without reparsing
every candidate. The make target uses the native script runner and validates
the generated report so the queue stays machine-consumable for release gates
and future UI surfaces.
Readiness also carries validation profile mappings. A candidate with only
`component_profile` entries is not broken, but it is still weaker than a
candidate with a `release_candidate` validation profile because the executable
validation lane has not yet been promoted to the same granularity as the
release qualification record.
Release-retained artifacts also carry `release_review_status` and
`release_review_gate` in readiness output, plus a retained decision path when
the release record has one. This keeps pending reviewer sign-off and
scope-blocked screening claims visible without promoting any operator trust
level prematurely.
The readiness summary rolls these into `release_review_statuses`, so release
owners can see pending sign-off, approved, rejected, missing, and scope-blocked
release artifacts without scanning every candidate.
It also reports `release_review_decisions`, which counts required, declared,
retained, and missing review decision records for release-retained artifacts.
The same summary includes `operator_trust_levels`, so UI and CI surfaces can
show the current manifest distribution without reparsing reliability shards.
`make check-operator-reliability` builds and validates this readiness report
before checking the release manifest, so the qualification queue stays visible
without pretending that planning artifacts are qualification evidence.

## CFD Stokes Screening Scope

`solve.stokes_flow_quad_2d` is a Stokes-only screening operator. It is meant to
exercise low-Reynolds-number velocity, pressure, divergence, and viscous
dissipation plumbing through the same headless workflow path as the other
physics operators. It is not a general Navier-Stokes solver, turbulence model,
compressible-flow solver, or industrial CFD validation claim.

The current qualification evidence covers compact quad and triangle fixtures:
the quad lane checks body-force and lid-driven shear responses, while the
triangle lane checks geometry rejection and heterogeneous viscosity response.
The retained mesh-refinement regression uses a linear Stokes field on coarse
and refined quad/triangle meshes to verify stable area, divergence, shear,
velocity, pressure-drop, and viscous-stress diagnostics. That is enough to
qualify the Stokes screening boundary, but not enough to claim general CFD,
Navier-Stokes, turbulence, compressible-flow, or industrial design accuracy.

## CFD Stokes Divergence Tolerance

The screening divergence gate is `1e-10` for the current compact Stokes
fixtures. This tolerance is a regression guard for the single-quad arithmetic
path and boundary assembly, not a reusable engineering qualification tolerance.
The current screening-boundary qualification is backed by retained convergence
evidence, solver-version provenance, and a documented scope of validity. Any
future claim beyond that Stokes-only screening boundary must replace or extend
this tolerance policy.

The machine-readable screening policy lives at
`evidence/operator-qualification/stokes-flow-screening-tolerance-policy.json`.
That artifact pins the current regression scope and explicitly blocks using
the same tolerance for Navier-Stokes, turbulence, compressible-flow, or
mesh-convergence claims outside the retained linear-field fixture.

The CFD quality transform also exposes review-facing explanation fields:
`cfd_quality_dominant_term`, `cfd_quality_watch_count`, and
`cfd_quality_blocking_terms`. These fields make headless material or flow
screening runs explainable: a candidate does not only receive a score, it also
reports which diagnostic term dominated the penalty and which missing or
out-of-target terms caused a block. The same transform also accepts
`enabled_terms` and common alias fields such as `max_divergence_error`,
`max_reynolds_number`, `total_viscous_dissipation`, `velocity_span`, and
`pressure_span` so retained screening studies can narrow their objective
without rewriting upstream diagnostics. Diagnostics also normalize compact CFD
post-processing names such as `vx`/`vy`, `p`, `div_u`, `reynolds`, and
`dissipation`, while quality scoring accepts aliases such as `div_u_peak`,
`re_peak`, `dissipation_total`, `speed_span`, and `p_span`.

## Electromagnetic Plane Review Scope

The 2D electrostatic and magnetostatic plane operators are now qualification
grade for the retained single-patch field-energy scope. They verify that
triangle and quad elements can report gradients, field strength, flux density,
stored energy, material-parameter scaling, and rotated orientation behavior
through the same headless workflow contract. They still do not claim broad
mesh convergence, coupled high-frequency electromagnetics, or production multiphysics
qualification.

The `electromagnetic-plane-patch` qualification packet has reviewer sign-off
over field-energy, material-provenance, and orientation evidence. Its retained
validation report executes the electrostatic and magnetostatic triangle and
quad review fixtures together and is attached at
`releases/qualification-evidence/2.0.0/electromagnetic-plane-patch-release-evidence.json`.
The matching review decision promotes `solve.electrostatic_plane_triangle_2d`,
`solve.electrostatic_plane_quad_2d`,
`solve.magnetostatic_plane_triangle_2d`, and
`solve.magnetostatic_plane_quad_2d`.

## Electromagnetic Plane Material And Energy Notes

The current fixtures assume positive scalar linear material parameters.
Electrostatic plane elements use permittivity; magnetostatic plane elements use
permeability. The stored energy diagnostics are regression evidence for this
linear material path, not a broad material-card validation claim.

Before qualification, these operators need material-card provenance for the
permittivity or permeability values and an energy-density tolerance derivation
that explains where the stored-energy comparison is valid. The current
evidence lives at
`evidence/operator-qualification/electromagnetic-plane-field-energy-derivation.md`
and
`evidence/operator-qualification/electromagnetic-plane-material-provenance.json`;
the orientation regression lives at
`workers/rust/crates/solver/tests/electromagnetic_plane_orientation_regression.rs`.
It remains review evidence until the qualification promotion is explicitly
signed off.

## Thermal Plane Review Scope

The 2D heat-plane and thermoelastic-plane operators are now qualification grade
for the retained compact patch scope. Heat-plane fixtures exercise steady
temperature gradients, heat-flux diagnostics, boundary coverage, and triangle
versus quad patch equivalence. Thermoelastic-plane fixtures exercise restrained
thermal strain, mechanical strain, stress, von Mises diagnostics, material
parameter provenance, and the same mesh/refinement equivalence. They still do
not claim arbitrary mixed-boundary heat-transfer coverage or production
thermo-mechanical qualification outside the retained patch envelope.

The `thermal-plane-patch` qualification packet has reviewer sign-off over
boundary coverage, material-parameter provenance, and mesh/refinement
equivalence. Its validation profile executes the heat and thermoelastic
triangle/quad review fixtures together, then checks that a two-triangle split
matches the quad patch response for heat flow and restrained thermal stress.
For the moxi 2.0.0 line, the retained validation report is attached at
`releases/qualification-evidence/2.0.0/thermal-plane-patch-release-evidence.json`.

## Thermal Plane Material And Boundary Notes

The current thermal fixtures assume linear material behavior. Heat-plane
elements use positive scalar conductivity. Thermoelastic-plane elements use
linear plane stress plus positive elastic constants and thermal expansion
coefficients. These material values are fixture parameters, not material-card
provenance.

The thermal plane qualification scope is backed by retained boundary and
material evidence at
`evidence/operator-qualification/thermal-plane-boundary-coverage.md` and
`evidence/operator-qualification/thermal-plane-material-provenance.json`; the
mesh/refinement regression lives at
`workers/rust/crates/solver/tests/thermal_plane_mesh_refinement_regression.rs`.

## Modal Frame Review Scope

The 2D and 3D modal-frame operators are review-grade cantilever modal checks.
They verify positive finite natural frequencies, mode ordering, period/frequency
conversion, restrained DOF zeroing, and expanded mode-shape normalization. The
current shape normalization contract uses a unit Euclidean participation norm
on the expanded shape vector.

The `modal-frame-sanity` qualification packet is now approved: it has a
normalization policy, a frequency-convergence note, and a regression that
checks 2D and 3D cantilever mode ordering plus the expected frequency increase
for shorter beams. Symmetric 3D bending modes may be near-degenerate, so the
3D ordering contract is non-decreasing rather than strictly increasing.
For the moxi 2.0.0 line, the retained validation report is attached at
`releases/qualification-evidence/2.0.0/modal-frame-sanity-release-evidence.json`.

The retained review decision promotes `solve.modal_frame_2d` and
`solve.modal_frame_3d` for the current linear modal cantilever scope. The
current modal evidence lives at
`evidence/operator-qualification/modal-frame-normalization-policy.md`,
`evidence/operator-qualification/modal-frame-frequency-convergence.md`, and
`workers/rust/crates/solver/tests/modal_frame_sanity_regression.rs`.

## Current State

The current `moxi 2.0.x` manifest covers all 38 solve operators in the
`physics-coverage` benchmark matrix, with a release gate requiring at least
`review` evidence for every covered operator.

Current level distribution:

- `baseline`: 0 operators
- `smoke`: 0 operators
- `review`: 5 operators
- `qualification`: 33 operators

This remains intentionally conservative. The platform has broad executable
coverage, and selected line-field, structural, thermal, electromagnetic,
modal, acoustic, transport, spring, and Stokes-screening subsets have crossed into
scoped qualification evidence.

The CFD-facing Stokes operators remain `screening_only` in scope, but the
`screening-cfd-boundary` evidence kit is now qualified for that boundary: the
quad lane has body-force and lid-driven shear boundary fixtures, the triangle
lane adds geometry rejection plus heterogeneous viscosity response, and the
mesh-refinement fixture verifies a linear Stokes field on coarse and refined
quad/triangle meshes. The test suite encodes a screening divergence tolerance
and the retained screening policy documents its current limits. The
`screening-cfd-boundary` validation profile now acts as the release-candidate
aggregation lane, while `stokes-flow-screening` remains the narrower component
profile. Future Navier-Stokes or industrial CFD claims still need separate
external-reference or benchmark evidence.

The acoustic 1D bar is now qualified for the retained linear frequency-domain
duct scope. Its closed-form evidence solves the reduced scalar pressure
response at two frequencies, checks wave number and particle velocity against
the analytic formulas, and verifies that an undamped fixture reports zero
damping loss. This does not claim branched duct networks, nonlinear acoustics,
transient propagation, or 3D acoustic cavities.

The advection-diffusion 1D bar is now qualified for the retained steady
constant-coefficient transport scope. Its closed-form evidence checks
diffusive, advective, and total flux across diffusion-dominant and
advection-dominant Peclet regimes, plus the zero-velocity limit where
advective flux must vanish. This does not claim transient transport, nonlinear
reaction, multidimensional flow, turbulent mixing, or arbitrary stabilization
schemes.

The magnetostatic 1D bar is now qualified for the retained linear single-core
permeance scope. Its closed-form evidence checks magnetic potential, field
strength, flux density, and stored energy across length, area, permeability,
and source scaling, plus the zero-source limit. This does not claim nonlinear
magnetic materials, hysteresis, saturation, eddy currents, or time-varying
fields.

The spring 1D chain is now qualified for the retained linear static series
scope. Its closed-form evidence checks equivalent series stiffness, member
force continuity, element strain energy, and the zero-load response. This does
not claim nonlinear springs, transient dynamics, contact, or arbitrary vector
spring networks.

The spring 2D and 3D operators are now qualified for the retained linear static
orthogonal vector-spring scope. Their closed-form evidence checks inverse
diagonal stiffness displacement, fixed-support displacement, member force,
extension sign, and strain energy for planar and spatial spring projections.
This does not claim nonlinear springs, contact, transient dynamics, or general
mesh-convergence behavior for arbitrary spring networks.

The thermal beam 1D operator is now qualified for the retained linear
free-curvature scope. Its closed-form evidence checks thermal curvature, tip
rotation, tip displacement, zero-gradient response, and near-zero internal
force for a single fixed-free member. This does not claim thermal frame
assemblies, nonlinear material behavior, transient heat transfer, buckling, or
plasticity.

The thermal truss 3D operator is now qualified for the retained fully
restrained uniform-temperature scope. Its closed-form evidence checks zero
node displacement, thermal/mechanical strain split, compressive stress, axial
force, and member energy summation. This does not claim partial restraint,
temperature gradients, buckling, plasticity, contact, or dynamic response.

The thermal truss 2D operator is now qualified for the retained fully
restrained uniform-temperature scope. Its closed-form evidence mirrors the 3D
thermal truss lane by checking fixed node displacement, thermal/mechanical
strain split, compressive stress, axial force, and member energy summation.
This does not claim partial restraint, mixed thermal loading, temperature
gradients, buckling, plasticity, contact, or dynamic response.

The thermal frame 2D operator is now qualified for the retained fully
restrained uniform-temperature single-member scope. Its closed-form evidence
checks fixed end displacement and rotation, thermal/mechanical strain split,
axial force, axial stress, zero-gradient moment and shear response, and strain
energy. This does not claim partial restraint, temperature gradients, frame
assemblies, geometric nonlinearity, buckling, plasticity, contact, or dynamic
response.

The thermal frame 3D operator is now qualified for the retained fully
restrained single-member scope with uniform temperature and linear gradients.
Its closed-form evidence checks fixed end translations and rotations,
thermal/mechanical strain split, thermal curvatures, axial force, bending
moments, combined stress, and strain energy. This does not claim partial
restraint, arbitrary 3D frame assemblies, torsion-dominant response, geometric
nonlinearity, buckling, plasticity, contact, or dynamic response.

The contact gap 1D operator is now qualified for the retained penalty stop
scope. Its closed-form evidence checks inactive gap response, active penalty
penetration, contact activation count, spring force, contact force, and
force-split equilibrium. This does not claim multidimensional contact,
friction, impact, large deformation, or industrial contact search.

The truss 2D operator is now qualified for the retained symmetric two-bar
scope. Its closed-form evidence checks fixed supports, apex symmetry, vertical
displacement, equal axial member force, stress, strain, and strain energy.
This does not claim arbitrary truss topology, geometric nonlinearity, buckling,
dynamic response, damaged members, or 3D space truss behavior.

The truss 3D operator is now qualified for the retained symmetric tripod scope.
Its closed-form evidence checks fixed base supports, zero lateral apex motion,
vertical displacement, equal axial leg force, stress, strain, and strain
energy. This does not claim arbitrary space-frame topology, geometric
nonlinearity, buckling, damaged members, joint eccentricity, or dynamic
response.

The first qualification evidence collection track, `line-field-closed-form`,
is now approved for qualification. Its versioned baseline artifact lives at
`evidence/operator-qualification/line-field-closed-form-baseline.json` and is
paired with
`evidence/operator-qualification/line-field-closed-form-derivation.md` plus
`evidence/operator-qualification/line-field-tolerance-policy.json`. These are
checked by `make check-line-field-closed-form-baseline`. This pins the
closed-form expected values, tolerances, and tolerance scope for `solve.bar_1d`,
`solve.thermal_bar_1d`, `solve.heat_bar_1d`, and
`solve.electrostatic_bar_1d`; those four operators now carry
`evidence.qualification` entries in the reliability shards.
`make capture-line-field-qualification-provenance` can emit the release-time
revision, toolchain, platform, and input-hash envelope without adding local
machine paths to Git. `make capture-line-field-qualification-release-evidence`
runs the evidence checker and solver baseline and writes the release-retained
regression bundle. The bundle now includes a `promotion_summary` tying the
approved review decision, release record, and four promoted operator ids to the
same retained evidence path. For the moxi 2.0.0 line, that retained bundle is
attached at
`releases/qualification-evidence/2.0.0/line-field-closed-form-release-evidence.json`
and referenced by `releases/qualification-records/1.20.0.json`. The retained
review decision at
`releases/qualification-review-decisions/2.0.0/line-field-closed-form-review-decision.json`
approves the promotion against the graduation gate. The release-record checker
also reads approved evidence bundles and requires their `promotion_summary` to
match the release record, review decision path, release version, and roadmap
operator IDs before an approved record can remain valid. The readiness report
summarizes that same gate as `summary.release_promotion_summaries`, currently
showing twenty-three approved promotions as retained, declared, matched, and not
missing.

`solve.solid_tetra_3d` is now qualified for the current unit constant-strain
tetrahedron scope. The retained evidence derives the reduced stiffness for a
three-node restrained base with one loaded free tip, then checks displacement,
constitutive stress components, von Mises stress, and strain energy against
`workers/rust/crates/solver/tests/solid_tetra_3d_closed_form.rs`. This remains
a single-element linear-elastic qualification, not a mesh-convergence,
plasticity, contact, or large-deformation claim.

`solve.nonlinear_spring_1d` is now qualified for the current single hardening
spring scope. The retained evidence derives the Cardano root for
`F = k u + c u^3`, then checks the Newton result, force balance, tangent
stiffness, residuals, and monotonic load-step factors against
`workers/rust/crates/solver/tests/nonlinear_spring_1d_closed_form.rs`. This is
a monotone one-dimensional hardening qualification, not a hysteresis,
softening, snap-through, or dynamics claim.

`solve.frame_3d` is now qualified for the current single-member cantilever
scope. The retained evidence derives the Euler-Bernoulli displacement, slope,
root moment, bending stress, and strain-energy formulas for an x-aligned 3D
frame, then checks them against
`workers/rust/crates/solver/tests/frame_3d_closed_form.rs`. This remains a
single-member linear static qualification, not a multi-member stability,
geometric nonlinearity, warping, plastic-hinge, or dynamics claim.

`solve.plane_triangle_2d` and `solve.plane_quad_2d` are now qualified for the
current small plane-stress patch scope. The retained evidence checks the
triangle direct-stiffness reference, the quad split-triangle weighted contract,
stress diagnostics, von Mises handling, and strain-energy totals against
`workers/rust/crates/solver/tests/plane_2d_closed_form.rs`. This remains a
small-patch qualification, not a mesh-convergence, high-order quadrature,
distorted-element, plasticity, buckling, or large-deformation claim.

The `beam-frame-classic` qualification candidate is now approved for
qualification. Its reference note is
`evidence/operator-qualification/beam-frame-classic-reference-note.md`, and its
first multi-case regression is
`workers/rust/crates/solver/tests/beam_frame_classic_regression.rs`. That test
checks a closed-form cantilever beam, equivalent 2D frame cantilever, and
prismatic torsion shaft. Its sign convention note is
`evidence/operator-qualification/beam-frame-force-sign-convention.md`. The
`beam-frame-classic` profile is also part of `make verify-operator-validation`,
so release validation output now executes the regression, beam review, torsion
review, and frame review fixtures together. For the moxi 2.0.0 line, that
retained output is attached at
`releases/qualification-evidence/2.0.0/beam-frame-classic-release-evidence.json`
and referenced by `releases/qualification-records/1.20.0.json`. Use
`make check-beam-frame-qualification-release-evidence` before relying on the
file; it rejects non-executed reports, mixed-profile reports, missing evidence
paths, and failed beam/frame/torsion commands. Its retained review decision
approves promotion against the graduation gate, so `solve.beam_1d`,
`solve.torsion_1d`, and `solve.frame_2d` now carry `evidence.qualification`
entries in the structural reliability shard.

## Smoke-Level Gaps

There are currently no smoke-only operators in `physics-coverage`.

## Upgrade Rules

Do not raise a solver trust level by editing the label alone.

To move from `smoke` to `baseline`, the manifest should point to at least one
of:

- an accuracy-baseline test with an explicit baseline function name
- a focused reliability test suite
- a benchmark profile that can catch performance or result-shape regressions

To move from `baseline` to `review`, add:

- documented assumptions and limitations
- mesh, geometry, boundary, or material preflight evidence where relevant
- tolerances or comparison criteria that are meaningful to the domain
- reportable diagnostics that a human reviewer can inspect
- a `review` evidence block in the manifest with assumptions, boundary checks,
  diagnostics, and focused tests

To move from `review` to `qualification`, add:

- external-tool, literature, analytic, or convergence evidence
- versioned baseline provenance
- release-blocking regression checks
- a documented scope of validity
- an `evidence.qualification` block with validation sources, convergence
  checks, provenance, release gates, and focused tests

`production_qualified` is deliberately outside the `1.x` target. That level
requires process controls and domain-specific validation that should not be
implied by the current screening and baseline stack.

## Near-Term Push

The most useful next upgrades are:

- harden selected review operators toward `qualification` with external,
  convergence, or literature-backed evidence
- use the approved `line-field-closed-form` packet as the template for the
  next small, analytic qualification candidates
- add mesh, boundary, and material-assumption evidence where review coverage is
  still based on compact screening fixtures
- keep Stokes-flow qualification scoped to the retained screening-boundary
  convergence fixture until a stronger CFD benchmark or reference-tool
  comparison exists
- keep future qualification promotions blocked until external, convergence,
  literature, or analytic evidence exists
