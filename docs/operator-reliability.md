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

The 2026-09-23 [multiphysics output-range regression](../reports/multiphysics-output-reliability-20260923.md)
adds bounded numerical-range checks to the plane-mechanical, solid-tetra,
electromagnetic-plane and Stokes screening profiles. It covers analytical
stress/field magnitudes, mechanical output rejection, cancellation and clean
in-process headless replay. These checks do not qualify nonlinear mechanics,
general CFD, installed transport or arbitrary floating-point input ranges.
The tensor records this as scoped `verified` evidence, not a blanket maturity
upgrade for the operator families.

For `daji 3.x`, the manifest also declares
`minimum_coverage_level: qualification`. `make check-operator-reliability`
treats this as a release gate, so future edits cannot silently downgrade a
covered operator back to `review`, `baseline`, or `smoke`. The Make target runs
the checker self-test first to keep the trust-level ordering and release-gate
behavior from regressing.

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

Rust workflow execution is still fail-fast by default. A non-core node can opt into
node-level recovery with `config.on_error: "skip"` or
`config.recovery.on_error: "skip"`; `fail` is also accepted as an explicit
fail-fast policy. Other recovery policy values are rejected during workflow
security preflight. When a recoverable node returns an error or panics, the run
records the node as `failed`, preserves its error message in `node_runs`, rolls
back artifacts written by the failed node, skips downstream nodes that cannot
resolve artifacts, and continues independent branches. This prevents
recoverable analysis/reporting failures from cascading across the whole graph
without hiding the failure from SDK or GUI callers.

Elixir Orchestra now implements the same explicit policy for returned operator
errors and validates declared policies before execution. It does not catch
arbitrary exceptions, process exits or workflow cancellation as recoverable
operator errors. Its graph responses always retain `failed_nodes` and compact
`node_failures` receipts, even when detailed traces and artifacts are omitted.
See the bounded branch-recovery contract below for evidence and limitations.

The qualification roadmap lives at
`config/operator-qualification-roadmap.json`. The same checker validates that
roadmap candidates reference existing manifest operators, already satisfy the
roadmap's minimum candidate level, and never set that candidate minimum below
the manifest's release-gated `minimum_coverage_level`.
Each roadmap candidate also carries a machine-readable qualification posture:

- `target_level`
  the trust level the candidate is trying to reach. Release-gated roadmap
  candidates for `daji 3.x` target `qualification`; screening-only or
  exploratory families should stay outside this release candidate queue until
  they have a qualification path.
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
Readiness v2 additionally emits `numerical_validation_depth` for every
candidate. A complete candidate must retain reference, convergence,
robustness, and release evidence; `reference_note` is counted separately as an
independent reference signal. This prevents a compact closed-form fixture from
silently standing in for convergence or external correlation.
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
The retained reference note derives the compact Stokes-only field `u = y`,
`v = 0`, `p = 0`, with zero divergence and unit shear rate. The retained
mesh-refinement regression uses that field on 1x1, 2x2, 4x4, and 8x8
quad/triangle meshes to verify stable area, divergence, shear, velocity,
pressure-drop, viscous-stress, and total viscous-dissipation diagnostics. That
is enough to qualify the Stokes screening boundary, but not enough to claim
general CFD, Navier-Stokes, turbulence, compressible-flow, or industrial
design accuracy.
The same retained regression now checks material-parameter scaling for the
linear screening field: quad and triangle viscosity changes scale viscous
stress and dissipation while leaving the prescribed velocity and shear rate
unchanged, triangle thickness changes scale dissipation only, and density
changes scale Reynolds diagnostics without changing viscous stress.
The quad and triangle paths also check geometry scaling with the prescribed
velocity boundary held fixed, so element area scales with domain area, shear
rate and viscous stress scale inversely with length, and viscous dissipation
remains fixed under the same thickness. Every retained branch also re-derives
element area from node coordinates, element velocity gradients, divergence,
shear rate, viscous shear stress, Reynolds number, and viscous dissipation
from public node, element, and material fields.

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
stored energy, material-parameter scaling, thickness scaling, and rotated
orientation behavior through the same headless workflow contract. They still do
not claim broad mesh convergence, coupled high-frequency electromagnetics, or
production multiphysics qualification.

The `electromagnetic-plane-patch` qualification packet has reviewer sign-off
over field-energy, material-provenance, and orientation evidence. Its retained
validation report executes the electrostatic and magnetostatic triangle and
quad review fixtures together and is attached at
`releases/qualification-evidence/2.0.0/electromagnetic-plane-patch-release-evidence.json`.
The magnetostatic retained reliability suite also includes a linear-field
diagonal-invariance check: changing the two-triangle split preserves nodal
vector potential and flux density, while a permeability perturbation scales
vector potential, flux density, and stored energy but leaves magnetic field
strength stable for the same source density.
It now also verifies the Dirichlet manufactured potential `A_z = 5y` over
1x1, 2x2, 4x4, and 8x8 triangle and quad meshes. The recovered flux density
is `B_x = 5`, `B_y = 0`, and total stored energy remains analytic across the
refinement ladder. This is linear-field mesh evidence only; it does not expand
the qualification claim to arbitrary magnetostatic geometries or sources.
The matching electrostatic paths verify `V = 8x` over the same triangle and
quad refinement ladder, recovering `E_x = -8`, `D_x = -24`, and analytic
stored energy. Together these checks make the linear field-energy contract
independent of the plane element shape and refinement level.
The plane-orientation regression also perturbs element thickness. For the
Dirichlet electrostatic patch, field terms and energy density stay fixed while
total stored energy scales linearly with thickness. For the current-driven
magnetostatic patch, vector-potential gradient, flux density, and field
strength scale inversely with thickness; energy density scales with the inverse
square and total stored energy scales inversely.
The retained executable check now also re-derives the result summaries from
nodes and elements: maximum potential/vector potential, maximum field strength,
maximum flux density, maximum energy density, node id/coordinate/source
passthrough, element average potentials, field or flux orientation laws,
field/flux magnitudes, element area from node coordinates, material
constitutive laws, and stored-energy totals must all agree with the reported
diagnostics.
The matching review decision promotes `solve.electrostatic_plane_triangle_2d`,
`solve.electrostatic_plane_quad_2d`,
`solve.magnetostatic_plane_triangle_2d`, and
`solve.magnetostatic_plane_quad_2d`.

## Electromagnetic Plane Material And Energy Notes

The current fixtures assume positive scalar linear material parameters.
Electrostatic plane elements use permittivity; magnetostatic plane elements use
permeability. The stored energy diagnostics are regression evidence for this
linear material path, not a broad material-card validation claim.

The qualification packet retains material-card provenance for the permittivity
or permeability values and an energy-density tolerance derivation that explains
where the stored-energy comparison is valid. The current evidence lives at
`evidence/operator-qualification/electromagnetic-plane-field-energy-derivation.md`
and
`evidence/operator-qualification/electromagnetic-plane-material-provenance.json`;
the orientation regression lives at
`workers/rust/crates/solver/tests/electromagnetic_plane_orientation_regression.rs`.
The manufactured electrostatic and magnetostatic plane refinement tests now
close the same packet's convergence dimension.

### Scalar Reference And Split-Quad Energy

Heat, electrostatic and magnetostatic triangle/split-quad solvers share a scalar
element kernel. Prescribed values are shifted by a fixed reference before
constraint reduction; gradients are evaluated from relative values and nodal
differences. Absolute nodal values and element averages are restored only at
the result boundary. Adding a common constant must not create heat flux,
electric field, magnetic flux or stored energy. This cannot recover differences
already rounded away in the input `f64` values, and the reported absolute nodal
values remain limited by their floating-point resolution.

Split quads keep area-averaged gradient/field vectors for visualization, but
energy density is the area-weighted average of the two subtriangle densities:
`sum(area_i * coefficient * |grad_i|^2 / 2) / sum(area_i)`, with coefficient
`permittivity` for electrostatics and `1 / permeability` for magnetostatics.
Do not reconstruct energy by squaring the averaged vector: opposing fields can
have zero mean and positive stored energy. Total energy is density times area
times thickness. This is still the two-triangle discretization, not a new Q4
isoparametric element. Nonuniform quad energies may differ from older results
because the old squared-average formula understated them. Disposable development
outputs may be discarded without migration or bulk recomputation. Only results
that will still be used for this diagnostic need a fresh solve; retained results
are not silently rewritten. Keep the solver, example definitions and regression
tests as described in `data-lifecycle.html#development-retention`.

`workers/rust/crates/solver/tests/scalar_plane_reference_invariance.rs` covers
constant/linear skew patches, positive and negative reference shifts, nonzero
sources, heterogeneous coefficients, reversed connectivity, dense and sparse
solves, and equal/unequal-area energy integrals. The bounded local evidence and
precompute microbenchmark are recorded in
`reports/scalar-plane-kernel-20260920.md`; neither extends production material
qualification or proves an end-to-end/1M-node performance improvement.

## Thermal Plane Review Scope

The 2D heat-plane and thermoelastic-plane operators are now qualification grade
for the retained compact patch scope. Heat-plane fixtures exercise steady
temperature gradients, heat-flux diagnostics, boundary coverage, and triangle
versus quad patch equivalence. Thermoelastic-plane fixtures exercise restrained
thermal strain, mechanical strain, stress, von Mises diagnostics, material
parameter provenance, and the same mesh/refinement equivalence. They still do
not claim arbitrary mixed-boundary heat-transfer coverage or production
thermo-mechanical qualification outside the retained patch envelope.

The `thermal-plane-patch` packet retains reviewer sign-off over compact boundary,
material-provenance and refinement fixtures, not arbitrary meshes. Its profile
checks triangle/quad heat and restrained-stress equivalence. The [CST orientation repair](../reports/plane-triangle-kernel-20260927.md)
adds all-six node orders, mixed-orientation expansion and signed stress checks.
Previously computed clockwise/mixed-orientation mechanical or thermoelastic triangle results must be recomputed before reuse.
On the current daji 3.x line, `solve.thermal_plane_quad_2d` is a native
bilinear isoparametric Q4 rather than a pair of constant-strain triangles. It
uses full 2x2 Gauss integration for stiffness, nodal-temperature interpolation,
thermal equivalent loads, stress recovery, and strain-energy recovery; see [Q4 kernel validation and timing](../reports/plane-q4-kernel-20260927.md).
Distorted 1x1, 2x2, and 4x4 meshes retain exact free uniform thermal expansion;
a restrained distorted patch integrates a linear temperature field over the
physical element area; inverted Gauss-point Jacobians are rejected.
The retained heat-plane mesh regression also carries a linear-field manufactured
check: changing the triangle diagonal preserves nodal temperatures, gradients,
heat flux, and total heat-flow rate, while a conductivity perturbation scales
heat flux without changing the recovered temperature gradient. The same
manufactured field now perturbs thickness and verifies that temperature
gradient plus heat-flux density stay fixed while total heat-flow rate scales
linearly with thickness.
Quad and triangle heat-plane meshes also run the same manufactured linear
temperature field over 1x1, 2x2, 4x4, and 8x8 refinements, preserving nodal
temperatures, gradients, heat-flux density, and total heat-flow rate across
the refinement ladder. The retained executable check also re-derives maximum
temperature, maximum heat flux, average element temperature, Fourier heat-flux
components, element geometry area, element heat-flow rate, and total absolute
heat-flow rate from the reported node and element fields.
The retained thermoelastic patch checks temperature-delta and thickness scaling:
restrained stress is linear in temperature delta, energy is quadratic, and
thickness scales total energy without changing stress or energy density.
Public displacement, temperature, stress, strain, area and energy summaries are
re-derived. [Thermal output range and recovery](../reports/thermal-plane-output-reliability-20260927.md)
also checks compensated temperature means, stable norms, energy-volume products,
nonfinite-result rejection and cancellation/replay through the native SDK route.
Arithmetic stress tests do not extend the validated physical material envelope.
An independent triangle/quad refinement regression now applies the same fully
restrained uniform temperature rise on 1x1, 2x2, 4x4, and 8x8 meshes. It keeps
zero displacement, thermal strain, peak stress, energy density, and total
strain energy invariant across both discretizations. This is a uniform-field
thermoelastic proof point, not a claim for arbitrary coupled thermal boundary
conditions.
For the moxi 2.0.0 line, the retained validation report is attached at
`releases/qualification-evidence/2.0.0/thermal-plane-patch-release-evidence.json`.
That report remains historical evidence for the earlier implementation. The
native Q4 implementation is retained separately at
`releases/qualification-evidence/2.7.9/thermal-plane-q4-isoparametric-evidence.json`.

## Thermal Plane Material And Boundary Notes

The current thermal fixtures assume linear material behavior. Heat-plane
elements use positive scalar conductivity. Thermoelastic-plane elements use
linear plane stress plus positive elastic constants and thermal expansion
coefficients. These material values are fixture parameters, not material-card
provenance.

The thermal plane qualification scope is backed by retained boundary and
material evidence at
`evidence/operator-qualification/thermal-plane-boundary-coverage.md` and
`evidence/operator-qualification/thermal-plane-material-provenance.json`.
The native thermoelastic Q4 formulas are retained at
`evidence/operator-qualification/thermal-plane-q4-closed-form.md`; the
mesh/refinement regression lives at
`workers/rust/crates/solver/tests/thermal_plane_mesh_refinement_regression.rs`.

## Frame Equation-Balance Reliability

The 2026-09-24 [equation-balance regression](../reports/frame-equation-balance-reliability-20260924.md)
extends the support-load fix to a bounded mixed-free-equation case. A very
large load on a disconnected axial member previously hid residuals in a
small cantilever, including end moments. Three negative controls reproduced
changed displacements and false acceptance of an exhausted Newton budget.

The shared `EquilibriumMetric` now checks each free equation using its own
reference load and absolute `K_ij * u_j` terms, in addition to the retained
global free-load residual. Terms in a moment row therefore have moment units,
not an unrelated model-wide force scale. Absolute tangent terms preserve a
local cancellation scale for unloaded equations. The metric uses scaled sums
and rejects nonfinite terms, malformed dimensions and invalid free mappings.

Load control, arc length, parameter continuation and modal correction share
this guard. Backtracking freezes a metric from the current state and full
Newton predictor before trying any step length; a trial cannot relax its own
scale by increasing displacement or load factor. No extra tangent solve or
section integration is introduced by this metric. Storage is linear in the
number of DOFs and the traversal uses existing sparse tangent rows.

The `frame-equation-balance-local-reliability` profile covers disconnected
strong/weak loads, force/moment controls, frozen search scales, cyclic history,
failed-state replay and in-process Rust headless/workflow routes. Arc paths
are compared with independently load-controlled solutions at their achieved
load factors, not assumed to have unchanged arc parameterization.

This is an additional local convergence guard, not a displacement-error,
mesh-error or constitutive-accuracy certificate. The existing absolute and
load-factor floors remain; complete unit invariance, arbitrary reference-load
reparameterization, near-singular coupled-system accuracy and large-scale
performance qualification remain separate work. Public schemas and material
commit policy are unchanged.

## Frame Nonlinear-Equilibrium Reliability

The 2026-09-23 [nonlinear-equilibrium regression](../reports/frame-nonlinear-equilibrium-reliability-20260923.md)
fixes support-load contamination of `solve.frame_2d_p_delta` and
`solve.frame_2d_material_p_delta` convergence. The old check normalized a
reduced free-equation residual by the entire reference load vector. Large
loads at constrained DOFs could therefore certify an unequilibrated trial,
alter arc-length adaptation or prevent cyclic plasticity from being computed.

The shared normalization now requires the free-DOF map and derives its
reference scale only from those equations. Load-control convergence and
backtracking, arc-length correction, parameter continuation, and branch-modal
correction/backtracking all pass the same reduced mapping used by their
linear systems. Invalid map dimensions or indices fail closed; nonfinite
input remains invalid even when it belongs to a support.

Public schemas, solver tolerances, load-factor normalization floors and
history commit policy are unchanged. This removes support loads from the
scale; it does not remove them from the model or suppress their reactions.
The `frame-nonlinear-equilibrium-local-reliability` profile retains constrained
load negative controls, ordinary linearized paths, cyclic histories, failed
trial/replay checks, and in-process Rust headless and workflow routes.

This is bounded support-load isolation evidence. The subsequent equation-balance
guard above adds mixed-scale negative controls; neither check is a general
force/moment accuracy certificate. Length-unit nondimensionalization and
arbitrary reference-load/load-factor reparameterization remain separate work.
No general nonlinear qualification, remote execution or performance gain is
claimed here.

## Frame Initial-Stress Reliability

The 2026-09-23 [initial-stress regression](../reports/frame-initial-stress-reliability-20260923.md)
corrects the zero-load equilibrium preflight for
`solve.frame_2d_material_p_delta`. Previously, a global maximum of element
area times parent yield strength, with an absolute floor, could accept a
nonequilibrated initial stress. Fully overridden parent material defaults or
an unrelated member could alter acceptance. Comparing moments against that
force scale also made the decision depend on length units.

Initial assembly now retains both signed forces and local absolute section
contributions. Each node's free translational residual is normalized by its
incident force contributions; its free rotational residual uses the incident
end-moment contributions separately. The force scale is an orientation-free
bound from axial force and end moments divided by member length. Fiber sums
remain available even when their section resultants cancel. Constraint
reactions are not tested as free-equation residuals.

The relative preflight threshold remains `1e-9`, with no absolute force floor
or yield-strength denominator. Normalize before evaluating the translation
norm; reject nonfinite forces or scales before comparison. Errors identify
the node index, translation/rotation component, relative residual and local
scale. Both the reference buckling baseline and imperfect initial geometry
use this guard. Newton tolerances, material commit policy and public task
and result formats are unchanged.

The `frame-initial-stress-local-reliability` component profile includes
tiny uniform prestress, mixed-fiber overrides, supported-member isolation,
length-unit changes, self-equilibrated residual-stress controls, constrained
reactions, finite-range guards and in-process Rust headless rejection/replay.
This is a relative node-local force-norm check, not exact equilibrium or a
separate bound for each translational component. It does not certify global
accuracy, prestress-relaxation workflows, arbitrary residual-stress fields,
durable restart, remote execution or installed GUI behavior.

## Adaptive Fiber-Integration Reliability

The 2026-09-23 [adaptive fiber regression](../reports/frame-fiber-adaptive-reliability-20260923.md)
fixes unit-dependent and range-dependent error estimation for
`solve.frame_2d_material_p_delta`. The previous force/moment Euclidean norm
could hide a bending discrepancy beside a large axial force, underflow to
zero or overflow, and depend on unused parent material defaults.

Candidate comparisons now evaluate axial force and both end moments
separately, using each component's own absolute accumulation scale. A scaled
symmetric relative discrepancy avoids squaring large/small values or mixing
force and moment units. A point-count-scaled roundoff allowance and a local
`1e-12` cancellation denominator floor prevent roundoff from forcing unnecessary
quadrature promotion; neither changes the requested integration tolerance.
Every candidate's response and history must be finite before order selection,
with the failed order included in the error. The existing 2/3/4/8/12-point
rules, fixed history identities and 29 evaluated stations per fiber remain.

The serialized `longitudinal_integration_error` field is unchanged, but its
value now represents the maximum componentwise estimate. Do not compare its
numbers directly with historical mixed-unit Euclidean estimates. Reaching the
12-point cap may still leave the estimate above the requested tolerance; that
diagnostic remains visible. Newton convergence is not an integration-tolerance
certificate. This estimate does not bound global mesh error, tangent error or
damage-softening localization, and is not a formal forward-error proof.

The `frame-fiber-integration-local-reliability` component profile covers force
scales and length-unit changes, unused parent defaults, candidate rejection,
retained dense cyclic references, and real in-process Rust headless mixed-fiber
paths. No remote benchmark, installed-app check or material-family promotion
is claimed by this local evidence.

## Frame Material-History Reliability

The 2026-09-23 [material-history regression](../reports/frame-material-history-reliability-20260923.md)
strengthens `solve.frame_2d_material_p_delta` without adding physics to the
engine dispatcher or changing the Rust headless task/result protocol.

The bilinear kinematic return map cancels the plastic-modulus expressions
analytically: the plastic increment uses `(yield_excess / E) * (1 - r)`,
the backstress increment uses `yield_excess * r`, and the tangent is `E * r`.
Recovering stress on the updated yield surface avoids subtracting two large
trial values. Tests cover very small/large material scales and hardening
ratios near one; these are floating-point range controls, not material data.

Committed perfect-plastic points keep a zero loading tangent instead of being
mistaken for virgin elastic points. Unloading still uses the elastic tangent.
Virgin mixed-fiber sections report their area-weighted material moduli, not
the parent element's fallback modulus. Section forces, tangents, diagnostics
and all evaluated material histories must be finite before assembly, commit
or reporting, including inactive adaptive quadrature histories.

The `frame-material-local-history-reliability` component profile checks
cyclic analytical references, finite-difference tangents, failed-reversal
rollback after an already yielded step, owned/borrowed agreement, and real
in-process Rust headless plans through the engine route. A failed path remains
nonconverged and retains the last accepted material state; fresh replay is
tested, not durable restart from an exported material checkpoint.

This is local numerical-validation/recovery evidence only. It does not qualify
arbitrary cyclic plasticity, damage softening, fatigue, collapse, all scales,
remote Agent execution or installed GUI behavior. Existing solver tolerances
and absolute convergence floors are unchanged.

## P-Delta Path Reliability

The 2026-09-23 [P-Delta path regression](../reports/frame-p-delta-path-reliability-20260923.md)
checks the transition from buckling modes to `solve.frame_2d_p_delta` results.
Explicit imperfection vectors are directions: multiplying them by a nonzero
positive scalar does not change the requested amplitude in the tested range.
Translation normalization no longer squares dimensional input directly or
rejects a direction solely because its magnitude is below a fixed threshold.
Unrepresentable scaled DOFs and unavailable mode indices return errors.

Linearized P-Delta, corotational and arc-length results share finite,
scale-stable imperfection projection and translation-norm recovery. The
linearized path additionally checks each reduced equilibrium equation using
componentwise backward error before publishing `converged=true`. Its reported
residual retains `||Ku-f|| / max(||f||, 1)` with stable norm evaluation. The
banded route reuses its factor for up to three iterative corrections rather
than relaxing near-zero-row validation; unresolved failures remain errors. The
shared nonlinear residual rejects nonfinite inputs and avoids overflowing its
normalization denominator; its existing load scale and convergence tolerance
are otherwise unchanged.

The `frame-p-delta-local-path-reliability` component profile covers secant
amplification, shape and common stiffness/load scaling, numerical range,
invalid indices, local weak-equation negative controls and Rust headless
cancellation/replay. `StabilityStep` and `StabilityRecovery` are appended solver
observation stages, not persistent restart points. Cancellation after a completed
linearized step returns an error rather than a shortened successful path.

Extreme-amplitude tests are floating-point stress tests, not valid
large-deformation P-Delta research models. This evidence does not qualify all
nonlinear material histories, postbuckling branches, arbitrary condition numbers
or failure diagnostics. Task/result protocols and engine routing are unchanged.

## Buckling Assembly Reliability

The linear screening operators `solve.buckling_beam_1d` and
`solve.buckling_frame_2d` retain their existing generalized eigenproblem and
dense/sparse routes. The 2026-09-23
[buckling assembly regression](../reports/buckling-assembly-reliability-20260923.md)
corrects beam assembly to use global positive-x rotation conventions regardless
of endpoint numbering. Reversing individual edges no longer changes the
critical load or mode shape. Reference compressive-force signs are unchanged.

Frame preload activity now matches the positive compression actually assembled
into geometric stiffness, without a unit-dependent absolute force cutoff.
Recovery-scale roundoff is suppressed using each member's axial stiffness and
endpoint translation norms; the original signed axial force remains available.
Portal beams with only numerical axial noise stay inactive under common scaling
and rigid rotation, while an independently supported weakly loaded member is
not filtered out by a large load elsewhere in the model.
Uniformly scaling stiffness and reference loads together preserves load factors;
scaling only the reference load inversely scales the factor. Zero and tensile
reference states remain rejected in the tested compression-only scope.

The `buckling-assembly-local-reliability` component profile checks numbering,
coordinate reflection/rotation, heterogeneous members, Euler limits, restraint
failures, finite unit-normalized mode recovery, cancellation and Rust headless
error/replay. Element stiffness overflow reports its index. Beam and frame
mode recovery share the same scale-stable normalization and finite checks.
Reported residuals retain the existing reduced-eigensolver normalization;
this is not a new normalized backward-error certificate.

This is bounded local evidence, not nonlinear collapse, arbitrary preload
accuracy, a global lowest-mode guarantee or a new large-scale qualification.
The existing compression-only frame geometric-stiffness approximation and
near-zero preload roundoff limitations remain. Engine dispatch, SDK payloads
and result schemas are unchanged.

## Harmonic Spring Reliability

The linear `solve.harmonic_spring_1d` operator solves the requested frequency
samples of `(K - omega^2 M + i omega C) u = f`. The
2026-09-23 [harmonic spring regression](../reports/harmonic-spring-reliability-20260923.md)
checks finite displacement, derived velocity/acceleration and element-force
amplitudes before publishing a successful sweep. A failing frequency reports
its zero-based index and value; earlier frames do not become partial success.
Residual validation scales each free equation and its incident displacements,
avoiding overflowing denominators and the loss of independent soft equations
beside much stiffer components. Complex elimination remains an internal
solver module; engine dispatch and SDK contracts are unchanged.

The `harmonic-spring-local-reliability` component profile covers damped
resonance, independent analytical path/cycle decompositions, average input
power versus damping dissipation, numerical range, cooperative cancellation
and clean replay through the Rust headless route. Evidence is bounded and
in-process, not general mechanical qualification or a remote benchmark.
`peak_frequency_hz` is a peak among supplied samples, not a continuous
resonance search. Non-path networks retain the 512-free-DOF dense limit;
large-path overhead, near-singular conditioning, arbitrary subnormal products
and distributed recovery are not certified by these checks.

## Transient Spring Reliability

The constant-load linear `solve.transient_spring_1d` operator retains the
average-acceleration Newmark scheme and a reused sparse factorization. The
2026-09-23 [transient spring regression](../reports/transient-spring-reliability-20260923.md)
recasts time stepping in velocity increments, avoiding the subtraction of
nearly equal total displacements to recover acceleration. Per-step finite
energy validation is independent of `history_stride`; only snapshot storage
is sampled. An unrepresentable intermediate energy fails the solve even when
that step would not be saved.

The `transient-spring-local-reliability` component profile checks preload and
rigid-translation invariance, damped analytical time refinement, discrete work
and dissipation balance, cancellation/replay and constant-load continuation
from returned displacement/velocity through the Rust headless route. This is
bounded in-process evidence, not general transient FEM qualification or
durable distributed checkpoint/restart. The endpoint `max_force` remains a
final-state statistic; it is not a peak over the entire time history.

## Modal Frame Review Scope

The 2D/3D modal operators are review-grade cantilever checks for finite positive
frequencies, ordering, period conversion, restrained DOF zeroing and expanded
mode shapes with unit Euclidean participation norm.

The `modal-frame-sanity` qualification packet is now approved: it has a
linear generalized eigenproblem reference note, a normalization policy, a
frequency-convergence note, and a regression that checks 2D and 3D cantilever
mode ordering plus the expected frequency increase for shorter beams across
eigenvalue, rad/s, Hz, and period. The retained regression also verifies the
Rayleigh scaling reference: uniform stiffness and density scaling drive modal
frequency by `sqrt(stiffness / density)`, eigenvalue by
`stiffness / density`, and period by the inverse frequency factor while
preserving free DOFs and normalized participation. Every retained branch also
re-derives min/max frequency, total mass from element density, area, and
node-coordinate length, eigenvalue/rad/s/Hz/period consistency, mode index
order, expanded shape constraints, and participation norm from the retained
mode fields.
Symmetric 3D bending modes may be near-degenerate: their order is non-decreasing
rather than strictly increasing.
For the moxi 2.0.0 line, the retained validation report is attached at
`releases/qualification-evidence/2.0.0/modal-frame-sanity-release-evidence.json`.

The retained review decision promotes `solve.modal_frame_2d` and
`solve.modal_frame_3d` for the current linear modal cantilever scope. The
current modal evidence lives at
`evidence/operator-qualification/modal-frame-reference-note.md`,
`evidence/operator-qualification/modal-frame-normalization-policy.md`,
`evidence/operator-qualification/modal-frame-frequency-convergence.md`, and
`workers/rust/crates/solver/tests/modal_frame_sanity_regression.rs`.

The 2026-09-23 [modal spectrum reliability regression](../reports/modal-spectrum-reliability-20260923.md)
checks heterogeneous stiffness, component restraint rank, single-/multi-mode
agreement, cancellation and headless error/replay. Soft positive modes are not
filtered using the largest eigenvalue; Jacobi coupling thresholds are local
to diagonal pairs. Orphan nodes and component rigid motions are rejected, not
silently removed. Free-free analysis is not introduced. Small single-mode
problems can use a complete spectrum; general sparse iteration uses two probes.
Probes and residuals are not global lowest-eigenvalue certificates. Cantilever qualification stays scoped, without new material or large-mesh qualification.

The [modal assembly regression](../reports/modal-assembly-reliability-20261002.md) checks full mass/stiffness before reduction, balanced products and cancellation/replay; lumped mass and unit-Euclidean shape contracts are unchanged.
The [component-spectrum regression](../reports/modal-component-spectrum-reliability-20261002.md) preserves independently scaled blocks; only exact zero couplings permit splitting, and connected range loss fails explicitly.
The [mode-shape regression](../reports/modal-mode-shape-reliability-20261002.md) checks active-only scaling, vector rescaling and cancellable recovery.
The [axial-chain fast-path regression](../reports/modal-chain-fast-path-reliability-20261002.md) checks scale-relative admission and single-/full-spectrum agreement.
The [general tridiagonal regression](../reports/modal-tridiagonal-reliability-20261002.md) checks relative bisection and two-sided recovery; the [preparation follow-up](../reports/modal-tridiagonal-preparation-reliability-20261003.md) retains weak couplings, rejects asymmetric/range-loss inputs and checks preprocessing cancellation with independently verified public replay. The [complete-spectrum follow-up](../reports/modal-jacobi-convergence-reliability-20261002.md) fixes premature Jacobi stopping for resolved soft modes without relaxing residual gates.
The [repeated-mode subspace checks](../reports/modal-cluster-subspace-reliability-20261002.md) compare mass-weighted projectors, not arbitrary basis directions, and verify resolved splitting, mode-count truncation and bounded sparse-path membership.
The [connected sparse-modal checks](../reports/modal-connected-spectrum-reliability-20261002.md) address close-mode stalling. The [mass-coordinate inverse follow-up](../reports/modal-mass-inverse-reliability-20261002.md) verifies factor reuse/inner PCG; [complete-bending refinement](../reports/modal-complete-bending-reliability-20261002.md) covers 66 elements and [compensated residuals](../reports/modal-compensated-bending-reliability-20261002.md) cover 80. The [internal residual polish](../reports/modal-polished-bending-reliability-20261002.md) extends low/high/complete spectra to 96/100 elements and spatial paired modes to 100. The [published-shape follow-up](../reports/modal-published-shape-reliability-20261002.md) checks the actual physical output after normalization, with original mass and bounded coordinate relaxation without a new factor. The [Rust JSON follow-up](../reports/modal-json-roundtrip-reliability-20261002.md) fixes feature-dependent decimal readback and checks bit-preserving published shapes and sampled task digests. The [live Agent TaskIR follow-up](../reports/modal-agent-taskir-reliability-20261002.md) admits both modal Engine routes and verifies real local TCP numerical output, entrypoint/digest/authority guards, cancellation and same-connection recovery, not installed/remote qualification. The [vector cancellation follow-up](../reports/modal-vector-cancellation-reliability-20261003.md) checks 64-component scan, dot and update boundaries without regrouping reductions, and verifies public and live Agent replay without partial modes. The [final-validation follow-up](../reports/modal-final-validation-reliability-20261003.md) checks final spectrum/physical-shape norms, late-mode discard, sparse extreme-scale replay and live Agent recovery on the same TCP connection. These are separate validation boundaries; the original residual gates remain unchanged. The 128-element case still rejects unresolved spectra, not qualifies as a supported solve.

## Current State

The current `daji 3.x` manifest covers all 38 solve operators in the
`physics-coverage` benchmark matrix, with a release gate requiring
`qualification` evidence for every covered operator.

Current level distribution:

- `baseline`: 0 operators
- `smoke`: 0 operators
- `review`: 0 operators
- `qualification`: 38 operators

This remains intentionally conservative. The platform has broad executable
coverage, and selected line-field, structural, thermal, electromagnetic,
modal, acoustic, transport, spring, and Stokes-screening subsets have crossed into
scoped qualification evidence.

The dynamic spring 1D operators are retained for the single-DOF transient and
harmonic response scope. The transient branch checks Newmark single-step
response, load scaling, and undamped time-step refinement; every retained
branch also re-derives history maxima, kinetic energy, strain energy, final
node state, node id/coordinate passthrough, initial-state history fields,
contiguous default history step numbering, and final spring/damping force
diagnostics. The optional positive `history_stride` control keeps the initial
state, every selected interval, and the final state while peak displacement and
velocity remain evaluated across every computed step. A shared scalar-sample
budget rejects oversized retained histories before allocation.
The harmonic branch checks dynamic-stiffness amplitudes, damping response,
retained input frequency order, harmonic node id passthrough, fixed-node
zero-amplitude phase, global maxima, peak frequency,
per-frequency maxima, and velocity/acceleration amplitudes from the
frequency result fields. Its reduced free-DOF topology is analyzed once per
frequency sweep. Numbering-independent path forests use a row-scaled,
partial-pivoted complex tridiagonal solve with linear storage; this includes
segments split by interior constraints and isolated reduced DOFs. A retained
10,000-node shuffled-topology regression verifies that route, while a leading
zero dynamic diagonal regression verifies that a nonsingular undamped system
is pivoted rather than falsely rejected. True cyclic or branched reduced
networks keep the pivoted dense fallback, capped at 512 free DOFs before
allocation.

The CFD-facing Stokes operators remain `screening_only` in scope, but the
`screening-cfd-boundary` evidence kit is now qualified for that boundary: the
quad lane has body-force and lid-driven shear boundary fixtures, the triangle
lane adds geometry rejection plus heterogeneous viscosity response, and the
mesh-refinement fixture verifies a linear Stokes field on 1x1, 2x2, 4x4, and 8x8
quad/triangle meshes with material-diagnostic scaling on both element shapes.
The retained screening tests also re-derive Stokes summary and element
diagnostics from public node/element fields, including node
id/coordinate/body-force passthrough, velocity magnitude, pressure drop,
element area, velocity gradients, shear rate, viscous stress, Reynolds number,
and viscous dissipation.
The test suite encodes a screening divergence tolerance and the retained
screening policy documents its current limits. The
`screening-cfd-boundary` validation profile now acts as the release-candidate
aggregation lane, while `stokes-flow-screening` remains the narrower component
profile. Future Navier-Stokes or industrial CFD claims still need separate
external-reference or benchmark evidence.

The transient heat 1D bar is now retained for the single-free-node implicit
Euler lumped-capacity scope. Its closed-form regression checks each history
step, contiguous default step numbering, final time, final node temperatures,
element length from input coordinates, final heat flux, and thermal energy.
It shares the positive `history_stride`, mandatory final-frame, and bounded
history-allocation contract used by transient spring analysis.
Every retained branch also re-derives history maxima and energies from lumped
capacity, checks that the last history frame matches final summary fields, and
recomputes final element average temperature, gradient, and Fourier heat flux.

The acoustic 1D bar is now qualified for the retained linear frequency-domain
duct scope. Its closed-form evidence solves the reduced scalar pressure
response across an octave frequency ladder, checks angular frequency, wave
number, particle velocity, and wave-number frequency linearity against the
analytic formulas, and verifies that an undamped fixture reports zero damping
loss. The material perturbation regression also checks that bulk
modulus and density changes drive speed of sound and wave number by the
expected square-root scaling while pressure response, particle velocity, and
damping loss remain closed-form matched. It also perturbs duct length and
requires the dynamic pressure, pressure gradient, particle velocity, and
damping loss to keep matching the same closed-form dynamic-stiffness reference.
The source-amplitude regression uses a pure-source fixture to verify linear
pressure/particle-velocity scaling and quadratic acoustic-intensity/damping-loss
scaling. The retained refinement regression also runs a fixed-pressure linear
field over 1, 2, 4, 8, and 16 elements, requiring pressure, pressure gradient,
particle velocity, and wave number to stay invariant under subdivision. Every
retained branch also re-derives summary maxima, sound-pressure level, speed of
sound, wave number, particle velocity, acoustic intensity, damping loss,
element length, node coordinate/source passthrough, and material echo fields
from node, element, and material fields. This does not claim branched duct
networks, nonlinear acoustics, transient propagation, or 3D acoustic cavities.
The fixed-pressure linear manufactured field also runs through 1, 2, 4, 8, and
16 duct elements, preserving nodal pressure, pressure gradient, particle
velocity, and wave number. It is a one-dimensional field-recovery check, not a
claim of mesh convergence for resonant, branched, or transient acoustics.

The advection-diffusion 1D bar is now qualified for the retained steady
constant-coefficient transport scope. Its closed-form evidence checks
diffusive, advective, and total flux across diffusion-dominant and
advection-dominant Peclet regimes, plus the zero-velocity limit where
advective flux must vanish. The retained material/flow perturbation regression
also verifies that fixed-boundary concentrations stay unchanged while
diffusivity scales diffusive flux and inversely scales Peclet number, and
velocity scales advective flux and Peclet number. It also checks length scaling:
the fixed-boundary gradient and diffusive flux scale inversely with length,
Peclet number scales linearly with length, and the advective flux remains
fixed by the same average concentration. The source-response regression adds a
three-node free concentration DOF and verifies that internal
source strength scales the middle concentration linearly while cross-sectional
area inversely scales the source-driven concentration increment. It also checks
the source-balance jump between left and right total flux against source per
area, so internal source terms cannot silently lose conservation. Every retained
branch now also re-derives summary maxima from node/element results and checks
element length, node coordinate/source passthrough, average concentration,
concentration gradient, `diffusive_flux = -D * grad(c)`, `advective_flux =
velocity * c_avg`, `total_flux = diffusive_flux + advective_flux`, and the
Peclet formula. This does not claim transient transport, nonlinear reaction,
multidimensional flow, turbulent mixing, or arbitrary stabilization schemes.
The zero-velocity manufactured linear concentration field now runs through
1, 2, 4, 8, 16, and 32 elements. It preserves every nodal concentration,
element gradient, diffusive flux, and total flux while keeping Peclet and
advective flux at zero. This is a pure-diffusion refinement and conservation
proof point; advection-dominant convergence remains separately scoped.

For simple 1D path topologies independent of node numbering, the heat,
electrostatic, magnetostatic, thermal, advection-diffusion, and acoustic bar
solvers use constrained tridiagonal direct paths. Interior prescribed values
may split the reduced matrix into a path forest without losing that sparse
route. Truly branched or cyclic inputs remain on their established bounded
sparse or dense fallback paths. This keeps the specialization topology-scoped
while making million-node chain studies bounded by linear storage and solve
work rather than iterative convergence.

The magnetostatic 1D bar is now qualified for the retained linear single-core
permeance scope. Its closed-form evidence checks magnetic potential, field
strength, flux density, and stored energy across length, area, permeability,
and source scaling, plus the zero-source limit. The retained regression also
checks the magnetic energy conjugacy
`stored_energy = 0.5 * magnetomotive_source * magnetic_potential`, summary
stored-energy summation, magnetic field/flux recovery, max magnetic potential,
element length, node coordinate/source passthrough, average magnetic potential,
magnetic-potential-gradient recovery, and the source balance
`magnetic_flux_density * area + magnetomotive_source = 0`. This does not claim
nonlinear magnetic materials, hysteresis, saturation, eddy currents, or
time-varying fields. The fixed-potential manufactured linear field now runs
through 1, 2, 4, 8, and 16 elements and preserves nodal magnetic potential,
potential gradient, magnetic field strength, flux density, and stored energy.

The spring 1D chain is now qualified for the retained linear static series
scope. Its closed-form evidence checks equivalent series stiffness, member
force continuity, element strain energy, and the zero-load response. The
retained scaling regression also verifies that load scaling linearly scales
displacement and member force while scaling strain energy quadratically, and
that uniform stiffness scaling inversely scales displacement and strain energy
while preserving series force continuity. It also perturbs node spacing to
verify that reported element length changes do not alter displacement, member
force, or energy for fixed discrete spring stiffness. Every retained branch
also re-derives element extension, member force, element strain energy,
node id/coordinate passthrough, `max_displacement`, `max_force`, and
`total_strain_energy = sum(element.strain_energy) = 0.5 * sum(F*u)` from public
input, node, and element fields.
The equivalent-chain refinement ladder now splits the same retained spring into
1, 2, 4, 8, 16, and 32 equal series elements while preserving tip displacement,
member force, element extension, and total strain energy. This does not claim
nonlinear springs, transient dynamics, contact, or arbitrary vector spring
networks.

The spring 2D and 3D operators are now qualified for the retained linear static
orthogonal vector-spring scope. Their closed-form evidence checks inverse
diagonal stiffness displacement, fixed-support displacement, member force,
extension sign, and strain energy for planar and spatial spring projections.
The retained vector-spring scaling regressions also verify that load scaling
linearly scales free-node displacement and member force while scaling strain
energy quadratically, and that uniform stiffness scaling inversely scales
free-node displacement and strain energy while preserving reaction/member force.
They also perturb orthogonal anchor distances to verify that reported element
length changes do not alter displacement, member force, or energy for a fixed
discrete spring stiffness. Every retained branch also checks
node id/coordinate passthrough, direction-cosine displacement projection,
`force = stiffness * extension`, element strain energy, `max_displacement`,
`max_force`, and
`total_strain_energy = sum(element.strain_energy) = 0.5 * sum(F*u)` from public
input, node, and element fields.
The retained orthogonal-axis refinement ladder now splits each 2D and 3D axis
spring into 1, 2, 4, 8, and 16 equal series elements while preserving free-node
displacement, member force, strain energy, and axis-projected node displacement.
This does not claim nonlinear springs, contact, transient dynamics, or general
mesh-convergence behavior for arbitrary spring networks.

The thermal beam 1D operator remains qualified for the retained linear
free-curvature scope. Closed-form checks cover root constraints, thermal
curvature, tip rotation/displacement, zero-gradient response, and near-zero
internal force and energy. Gradient and expansion scale the free response
linearly; section depth scales it inversely. Length scales rotation linearly
and displacement quadratically while preserving curvature. The `1/2/4/8/16`
refinement ladder retains the quadratic displacement and linear rotation
fields. Node identity/coordinates, maxima, and summed element energy are checked.
This does not qualify thermal frame assemblies, nonlinear materials, transient
heat transfer, buckling, or plasticity.

The bounded mechanical/thermal beam output regressions now also cover uniform
loads on cantilever, pinned-pinned and fixed-fixed members, restrained thermal
curvature, mixed loads, nonuniform lengths and piecewise stiffness. Element
energy integrates the equilibrium-recovered moment field `M(x)^2/(2EI)`,
including the uniform-load particular solution; neither end-force work nor
the cubic nodal interpolant alone is the reported physical field energy.
`max_moment` and `max_bending_stress` include interior extrema. Displacement
and rotation maxima remain **nodal** samples, not continuum extrema: a single
pinned-pinned element can have zero nodal deflection and nonzero bending energy.
Connectivity reversal preserves global `uy/rz` and endpoint action identities.
Unrepresentable stiffness, loads, recovered fields and energy totals fail rather
than serialize nonfinite values; preparation, assembly and result stages are
cooperatively cancellable and valid calls replay after failure/cancellation.
The Rust headless batch-to-engine route uses the same kernel without a protocol
change. These checks do not expand the retained qualification scopes or prove
remote installed-Agent behavior. See
[beam output regressions](../reports/beam-output-reliability-20260927.md).

The thermal truss 2D and 3D operators are qualified only for the retained fully
restrained uniform-temperature scope, with the planar restrained triangle
mirroring the spatial lane. Closed-form evidence checks zero displacement,
thermal/mechanical strain split, compressive stress, axial force, and energy.
Temperature and thermal expansion linearly scale strain, stress, and force,
and quadratically scale energy. Young's modulus scales stress, force, and
energy without changing thermal strain; area scales force and total energy
without changing stress or energy density. Uniform geometry scaling changes
length and total energy linearly, leaving strain, stress, force, and density
fixed. Every branch re-sums `strain_energy_density * area * length`, checks
maximum energy density, and re-derives mean temperature, thermal/mechanical
strain, Hooke-law stress, axial force, all public maxima, and total energy.
Node id/coordinate/temperature passthrough is also checked. This qualification
does not cover general partial restraint, mixed thermal loading, temperature
gradients, buckling, plasticity, contact, or dynamics.

The separate [thermal-truss output regression](../reports/thermal-truss-output-reliability-20260927.md)
adds stable temperature means/norms, balanced energy-volume products, and
finite assembly/recovery checks even behind fully fixed supports. The existing
25%-of-extent displacement heuristic now uses actual model bounds rather than
an implicit origin/unit extent. Entry, terminal, and 64-item cancellation plus
replay are checked, with in-process Rust headless failure propagation and
Linux release regressions. These are bounded `verified` numerical/recovery
claims, not an expansion of the qualified physical scope or a performance,
installed-Agent, nonlinear, or general rotation-invariant limit claim.

The thermal frame 2D qualification remains the retained fully restrained,
uniform-temperature single-member scope. Its closed forms check zero fixed
translations/rotations, thermal/mechanical strain, axial force/stress,
zero-gradient bending/shear and energy. Temperature and expansion scaling give
linear force/stress and quadratic energy; area scales force/energy but not
stress; modulus scales force/stress/energy but not strain; length scales energy
but not strain/stress/force. Each branch re-derives all public summaries and
member energy totals, checks node identity/coordinate/temperature passthrough,
and rejects non-finite coordinates, loads and temperatures. This qualification
does not extend to general partial restraint, gradients, assemblies,
nonlinearity, buckling, plasticity, contact or dynamics.

The separate [planar-frame output regression](../reports/frame-2d-output-reliability-20260927.md)
repairs the thermal energy's missing curvature-variation term. A single-element
transversely loaded cantilever previously lost 25% of its bending energy.
Mechanical and thermal 2D frames now share checked field-energy recovery,
finite full-matrix/load validation, stable temperature means/displacement norms,
and cooperative assembly/recovery cancellation. Closed forms, nodal work,
refinement, thermal superposition, connectivity reversal and a planar 3D
reduction cross-check the fix; raw cancellation and Rust headless failures
must not become successful results and fresh inputs must replay normally.
Thermal gradients remain element-local: reversing connectivity also reverses
the local-y gradient for the same physical field. Public schemas and nodal
maximum sampling are unchanged. These are bounded `verified` numerical and
recovery claims, not general 3D qualification or installed-Agent/performance
evidence. Previously affected energy results need recalculation.

The thermal frame 3D qualification remains the retained fully restrained
single-member scope with uniform temperature and linear gradients. Its closed
forms check zero fixed translations/rotations, thermal/mechanical strain,
both curvatures, axial force, bending moments, combined stress and energy.
Temperature, gradient and expansion scaling give linear strain/force/moment
and quadratic energy; modulus scales force/moment/energy without changing
strain/curvature; inertia scales bending moment/energy but not curvature/axial
force; length scales energy but not strain/force/moment. Each branch re-derives
all public summaries, member energy totals and node passthrough fields. This
qualification does not extend to arbitrary assemblies, partial restraint,
torsion-dominant response, nonlinearity, buckling, plasticity, contact or dynamics.

The separate [spatial-frame output regression](../reports/frame-3d-output-reliability-20260927.md)
replaces cancellation-prone thermal-work subtraction with positive axial,
torsional and dual-bending field-energy terms. Checked mechanical/thermal
recovery retains small elastic responses under free thermal expansion,
finite temperature averages, tiny displacement/rotation norms and explicit
numeric-range errors. Scale-first direction normalization prevents large
finite axes, directional springs and exact constraints from losing their
orientation. Full assembly checks run before constraints remove rows; result
collection and constraint projection observe cooperative cancellation.
Closed forms, work, refinement, rotation, reversal and Rust headless replay
support bounded `verified` claims, not general 3D qualification, installed-Agent
or performance claims. The length/direction floors and nodal maxima remain;
nearly parallel section axes are tested relative to a normalized direction.
Affected saved energy and scaled-direction results need recalculation.

The [spatial-support regression](../reports/frame-3d-support-reliability-20260927.md)
uses a reorthogonalized direction basis and QR reaction recovery instead of
normal equations. Near-parallel independent supports retain analytic reactions;
dependent or overcomplete blocks fail before constructing the free map.
Thermal/spring equivalence, energy-range checks and raw cancellation/replay
remain bounded numerical/recovery evidence, not broader physical qualification.

The [section-orientation regression](../reports/frame-3d-orientation-reliability-20260927.md)
restores an orthonormal, right-handed frame for accepted near-parallel hints.
Rigid-mode, thermal expansion, refinement and headless checks retain the
parallelism floor and legacy implicit-roll branch, without broadening qualification.

The contact gap 1D operator is now qualified for the retained penalty stop
scope. Its closed-form evidence checks inactive gap response, active penalty
penetration, contact activation count, spring force, contact force, and
force-split equilibrium. The retained scaling regression also verifies that
active contact remains active when load and gap are scaled together, with tip
displacement, penetration, spring force, and contact force preserving the same
scale factor. It also verifies contact-normal-stiffness scaling against the
closed-form penalty force split, including reduced penetration and the updated
spring/contact force balance for the same load and gap. It also perturbs the
spring element length to verify that the active penalty force split remains
independent of reported geometry length for fixed discrete spring and contact
stiffness. Every retained branch also checks the penalty contact law directly:
penetration is `max(ux - gap, 0)`, contact force is normal stiffness times
penetration, active counts match active flags, `max_contact_force` is the contact
force maximum, and spring plus contact force balances the external load. Every
retained branch also re-derives spring length from input coordinates, spring
extension, spring force, tangent stiffness, node id/coordinate passthrough,
max displacement, max spring force, nonlinear solve residual bounds, and
monotone converged load-step metadata from
public result fields. The retained refinement ladder now splits the same
linear spring path into 1, 2, 4, 8, and 16 elements for both inactive and active
penalty-stop branches while preserving the displacement line, spring force,
penetration, contact force, and active branch count. This does not claim
multidimensional contact, friction, impact, large deformation, or industrial
contact search.

The 2026-09-24 [spring/contact numerical-range regression](../reports/nonlinear-spring-range-reliability-20260924.md)
adds finite-state guards to the shared spring and penalty assembly, including
zero-residual convergence and final output after an exhausted iteration budget.
It retains large-displacement active/inactive force splits, rejects penalty
overflow, and checks cancellation followed by clean in-process replay. This
does not add friction, contact search or a nonlinear globalization strategy.

Mechanical truss qualification remains limited to the retained symmetric
two-bar 2D and symmetric tripod 3D scopes. Their closed forms check fixed
supports, zero lateral apex motion, vertical displacement, equal member force,
stress, strain, and energy. Load scales displacement, force, and stress
linearly and energy quadratically. Area inversely scales displacement, stress,
and energy while preserving force. Young's modulus inversely scales
displacement and energy while preserving load-controlled force and stress.
Similar-geometry scaling changes length, displacement, and energy linearly,
leaving force and stress fixed. Both lanes check
`total_strain_energy = 0.5 * apex_load * apex_displacement`, reconstruct public
maxima and element states, verify node id/coordinate passthrough, and re-sum
global external work. Splitting each member into 1, 2, 4, 8, and 16 parallel
area partitions preserves apex displacement, stress, strain, summed member
force, and total energy. These scopes do not claim arbitrary truss/space-frame
topology, geometric nonlinearity, buckling, damaged members, joint eccentricity,
or dynamics; the 2D evidence alone does not qualify 3D behavior.

The [mechanical-truss output regression](../reports/truss-output-reliability-20260927.md)
adds finite stiffness/state/energy guards, stable norms, actual-coordinate
model extents, and borrowed-solver cancellation with replay. Owned/borrowed
parity and profiling errors are also checked. Mechanical and thermal trusses
share Solver-local numeric checks, not Engine-specific logic. In-process Rust
headless routes retain failure propagation and small representable results.
This is bounded `verified` numerical/recovery evidence, not a new nonlinear,
large-mesh, installed-service or performance qualification.

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
`evidence.qualification` entries in the reliability shards. The axial bar now
has `bar_1d_tracks_load_area_and_modulus_scaling`, which verifies load
linearity, area-inverse displacement/stress/energy response, and
modulus-inverse displacement/energy response while preserving load-controlled
stress and axial force; it also verifies length-linear displacement and energy
response while preserving stress, strain-energy density, and axial force. Its
closed-form regression now also checks that summary strain energy equals the
element energy-density integral and the work-conjugate value
`0.5 * tip_force * tip_displacement`, while re-deriving tip displacement,
reaction force, node index/coordinate mesh passthrough, element index/endpoints,
element strain, stress, axial force, strain-energy density, and summary maxima
from public node and element fields. The thermal bar now has
`thermal_bar_1d_tracks_restrained_uniform_rise_scaling`, which verifies
temperature, thermal-expansion, modulus, area, and length scaling for the
fully restrained uniform-rise scope. Its retained regression re-derives element
length, node coordinate/temperature-delta passthrough, average temperature
delta, thermal strain, mechanical strain, total strain, stress, axial force,
energy density, total strain energy, and summary maxima from public node,
element, and input fields. The
heat and electrostatic line fields also have source/material scaling
regressions:
`heat_bar_1d_tracks_heat_load_and_conductivity_scaling` verifies heat-load
linearity plus conductivity-inverse temperature response while preserving
source-controlled flux, and also checks area-inverse temperature, gradient,
and heat-flux response plus length-linear temperature response while preserving
gradient and heat flux for the same heat load. Its retained regression also
checks Fourier flux recovery and the end-load conservation balance
`heat_flux * area + heat_load = 0`, plus max temperature, max heat flux,
element length, node coordinate/heat-load passthrough, average temperature, and
temperature-gradient recovery from
public result fields; and
`electrostatic_bar_1d_tracks_charge_and_permittivity_scaling` verifies charge
linearity, quadratic stored-energy scaling, and permittivity-inverse potential
response while preserving source-controlled electric flux density; it also
checks area-inverse potential, field, flux-density, and stored-energy response
plus length-linear potential and stored-energy response while preserving field
and flux density for the same charge source. It also verifies summary
stored-energy summation, electric-field/flux recovery, max potential, element
length, node coordinate/source passthrough, average-potential and
potential-gradient recovery, the source balance `electric_flux_density * area +
charge = 0`, and the electrostatic energy conjugacy `stored_energy = 0.5 *
charge * potential`.
`line_field_convergence.rs` now adds the convergence dimension for the same
qualification scope: axial, thermal, heat, and electrostatic line fields are
rerun on 1, 2, 4, 8, and 16 element meshes and must preserve the same closed
form displacement, stress, force, temperature or potential gradient, flux, and
energy quantities.
`make capture-line-field-qualification-provenance` can emit the release-time
revision, toolchain, platform, and input-hash envelope without adding local
machine paths to Git. `make capture-line-field-qualification-release-evidence`
runs the evidence checker, solver baseline, and current convergence regression,
then writes the release-retained regression bundle. The bundle now includes a
`promotion_summary` tying the approved review decision, release record, and
four promoted operator ids to the same retained evidence path. For the moxi
2.0.0 line, that retained bundle is attached at
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
`workers/rust/crates/solver/tests/solid_tetra_3d_closed_form.rs`. The retained
scaling regression verifies that load scaling drives displacement, stress, von
Mises stress, and energy by the expected linear/quadratic factors, while
elastic-modulus scaling inversely changes displacement and energy without
changing load-controlled stress. It also verifies tip-height scaling through
volume, displacement, constant strain/stress recovery, and total energy, plus
base-area scaling where a wider restrained base increases volume while
reducing displacement, stress, von Mises stress, and total energy by the
inverse area factor. The retained checks also assert
`total_strain_energy = 0.5 * tip_load * tip_displacement` for the single free
tip DOF. Every retained branch also re-derives node id/coordinate passthrough,
node displacement magnitude, element volume from tetra coordinates, total
volume, von Mises stress, `0.5 * stress dot strain` energy density,
maximum summary fields, total strain energy, and external work-energy from the
public node and element fields. The retained solid tetra input reliability
regression rejects non-finite coordinates and loads, missing or duplicate
topology, zero-volume tetrahedra, invalid Young's modulus, and invalid Poisson
ratio values before release evidence is accepted. The current-line depth lane
adds a `1/2/4/8` structured solid patch with six tetrahedra per cell and
independently distributed face traction. Every level recovers the affine
Poisson-contraction displacement, uniaxial stress, volume, and energy while
public reaction, free-residual, and resultant-balance diagnostics close the
global equilibrium contract. The additive fields preserve deserialization of
older result JSON. A separate self-equilibrated pure-bending lane uses exact
linear end traction and nullspace-only anchors across `2/4/8/16` meshes. Its
quadratic displacement, linear stress, and strain-energy errors contract to
`3.08%`, `14.60%`, and `2.82%` on `24,576` tetrahedra while anchor reaction and
force imbalance remain negligible. This qualifies scoped non-affine
multi-element convergence and exposes the slower stress convergence of the
constant-strain basis. A deterministic `22%` interior warp repeats the
`4/8/16` ladder and contracts to `4.56%` displacement, `19.00%` stress, and
`3.99%` energy error while minimum mean-ratio quality remains above `0.2827`.
Public quality diagnostics expose per-element mean ratio, visible distortion
thresholds/counts, and a near-incompressible locking-risk term at `nu >= 0.45`.
The scale-relative degeneracy gate accepts well-shaped microscopic elements,
reports solvable severe distortion, and rejects numerical slivers. Topology
preflight rejects orphan nodes and computes a centered, scale-normalized
six-mode rigid-body restraint rank for every connected component. It rejects
both a rank-`5/6` hidden rotation and a floating second component before
factorization. Separately restrained components solve as one block system and
report their component count; remapped node and element indices preserve the
physical response. [Normalized geometry and Linux timing](../reports/solid-tetra-kernel-20260927.md) also cover
exact translations, finite-range rejection and cohesive-host reuse. General unstructured
meshing, broad connectivity families, stabilized near-incompressibility, plasticity,
contact, native body/surface load integration and large deformation remain outside the claim.

`solve.nonlinear_spring_1d` is now qualified for the current single hardening
spring scope. The retained evidence derives the Cardano root for
`F = k u + c u^3`, then checks the Newton result, force balance, tangent
stiffness, residuals, and monotonic load-step factors against
`workers/rust/crates/solver/tests/nonlinear_spring_1d_closed_form.rs`. The
retained scaling regression also verifies that scaling the linear stiffness,
cubic stiffness, and load together preserves displacement while scaling force
and tangent stiffness by the same factor. It now also checks the conservative
hardening potential `U = 0.5 * k * u^2 + 0.25 * c * u^4`, with force and
tangent stiffness matching the first and second displacement derivatives of
that potential. It also perturbs element length to
verify that the discrete hardening law keeps the Cardano root, force, and
tangent stiffness independent of reported geometry length. Every retained
branch also re-derives spring extension from node displacement, force,
tangent stiffness, node id/coordinate passthrough, max displacement, max force,
residual bounds, and monotone converged load-step metadata from public result
fields. The active convergence lane also runs the Cardano comparison across
load, linear-stiffness, and cubic-stiffness perturbations, while the boundary
lane rejects non-finite node data and zero-length elements before Newton
iteration. This is a monotone one-dimensional hardening qualification, not a
hysteresis, softening, snap-through, or dynamics claim.

The `nonlinear-spring-range-local-reliability` profile adds weighted cubic
evaluation so the tested finite force/tangent responses do not depend on
representability of unweighted displacement powers. Non-finite element,
assembled, residual or output values fail before success/serialization;
constrained loads do not enter free residual subtraction. Twelve public
regressions and five Rust headless plan-to-engine tests supplement the retained
analytic checks. Absolute tolerance and finite non-converged trial semantics
were retained in that round; this is not arbitrary-scale or general nonlinear accuracy
qualification. See the [bounded evidence and limits](../reports/nonlinear-spring-range-reliability-20260924.md).

The follow-up `nonlinear-spring-committed-recovery` profile replaces the
non-converged trial output with the last committed state for both spring and
gap contact. `achieved_load_factor` explicitly identifies that state's load;
missing legacy metadata remains unknown. Root `residual_norm` belongs to that
state, whereas a failed step's residual belongs to its last evaluated trial.
Always inspect `converged`: zero root residual at a partial load is not
full-target acceptance. `iterations` now counts Newton corrections; the final
allowed correction is checked, and an already equilibrated step uses zero.
Only two displacement buffers are retained, with no per-step full snapshots.
The benchmark adapter rejects partial completion rather than reporting fast
success. The [recovery evidence](../reports/nonlinear-spring-recovery-20260924.md)
also covers headless and workflow propagation, cancellation and clean replay.
This is not a durable checkpoint/resume API or an adaptive continuation claim.

The `workflow-result-admission` component profile closes the next boundary:
explicit `converged: false` cannot become a passing guard, a ready quality score,
or a winning pair comparison. Shared admission covers nine domain consumers,
the three generic result extractors, summary reduction/selection/validation,
and composite quality sources. The material-frame `stability_result.converged`
wrapper is checked as well. Invalid marker types fail closed; missing markers
remain accepted for existing metric-only contracts, not certified as converged.
Load factors have operator-specific scales and are reported rather than compared
against a universal target. Trial histories are not scanned.
Incomplete results can still be exported intact for diagnosis. The default
workflow fails at the rejecting consumer; explicit `on_error: "skip"` retains
independent work and raw result outputs without publishing a quality result.
Candidate-ranking rejection keeps the ranking incomplete and requires replanning.
See the [admission regression and limits](../reports/workflow-result-admission-20260924.md).
This is local result-consumption evidence, not nonlinear physics qualification
or a guarantee for custom transforms that remove status metadata themselves.

The `workflow-guard-validation` profile additionally rejects missing or invalid
metrics in all nine domain guards and paired benchmarks. Successful checked-rule
and criterion counts now correspond to complete evaluation, not entries that were
silently skipped. Invalid explicit options do not become defaults: comparisons,
severity, goals, positive weights and nonblank fields/labels are validated before
use. The existing `value` threshold alias remains valid, but conflicting aliases
are rejected. Default `gte`, `warn`, `min`, unit weight and `left`/`right` labels
apply only when the corresponding option is omitted. Invalid explicit numeric
fields cannot hide behind valid fallback aliases. Derived non-finite metrics,
overflowing deltas and score totals fail before a verdict can be serialized.
Candidate labels must be distinct after trimming and cannot use the `tie` sentinel.
The [guard evidence](../reports/workflow-guard-validation-20260924.md) includes
branch-local recovery and corrected-input replay. It does not certify the
completeness of every domain-specific metric reducer or quality-score configuration.

The follow-up `workflow-domain-quality-validation` profile closes quality-score
configuration and arithmetic across the same nine domains. Explicit term lists
must be nonempty, known and unique; unknown entries cannot silently disappear.
Targets must be finite and positive, weights and ready limits finite and
nonnegative. Defaults apply only to absent options; null top-level config still
means defaults. Known inactive overrides remain reusable but are also validated.
Positive targets are used exactly, without the former hidden `1e-12` target floor.
Zero weight remains supported, but cannot hide a missing or malformed metric.
An absent selected metric still returns an explicit blocking assessment; an
invalid canonical numeric value cannot be replaced with a healthy alias.
Ratios, penalties and totals are checked before JSON serialization, so overflow
cannot become `null`, disappear from a sum, and produce an excellent score.
The nine domain wrappers retain their defaults, optional terms and output fields
while sharing typed evaluation and aggregation. No physics-specific scheduling
rule is added to the engine. The [quality evidence](../reports/workflow-domain-quality-validation-20260924.md)
covers default fail-fast behavior, opt-in branch recovery, corrected-input replay
and missing-metric propagation through composite scoring. Score formulas,
the inverse-goal denominator regularization, grade cutoffs and domain heuristics
are not independently qualified by this contract validation; generic alias and
sample-array reducers still require their own completeness audit.

The `workflow-metric-integrity` follow-up checks the shared resolver used by
the nine domain guards, quality scorers and paired comparisons. The selected
canonical value or first present alias must be numeric and finite; invalid
data cannot fall through to a later healthy alias. Frequency-response extrema,
peak-response frequency, transient one-axis node extrema and modal frequency
bounds require every consumed row and field. Errors identify the source array,
index and field. Absent or empty selected collections remain unavailable rather
than switching to a different source. Explicit valid summaries retain precedence
over raw arrays; this is not an independent recomputation or provenance check.
Optional display metadata uses a separately named best-effort API, not the
checked decision path. The first-mode participation metric inspects the first
mode, not participation values from unselected modes.

The Stokes/CFD diagnostic extractor also checks convergence admission and every
required velocity, pressure, divergence, Reynolds and dissipation sample. Both
node and element collections must be nonempty; incomplete collections cannot
invent zero diagnostics. Two streaming passes use constant auxiliary statistical
state instead of five temporary numeric arrays. Velocity magnitude uses `hypot`;
the mean no longer requires an unused raw sum to fit the numeric range. Derived
spans, means and dissipation totals must be finite before publication. The
[metric-integrity evidence](../reports/workflow-metric-integrity-20260924.md)
includes fail-fast, branch recovery and corrected-sample replay through CFD and
dynamic quality chains. The scoped audits below cover thermal, electrostatic,
magnetostatic, thermo-mechanical and transport diagnostics. The later
`workflow-field-bundle-integrity` profile extends the local Rust audit to generic
fields and diagnostic bundles. The scoped cross-runtime profile below checks
the four corresponding Elixir operations, not blanket runtime equivalence.

The `workflow-diagnostic-integrity` profile checks those three dedicated
diagnostic extractors through a shared sample-validation and reduction kernel.
The domain wrappers retain their physical field names and aliases; the solver
equations, engine scheduling and external SDK interfaces are unchanged. Explicit
nonconvergence is rejected before reduction. Both source collections must be
arrays and every record must be an object. A wholly absent optional metric group
is omitted, not filled with zeros; a group present in only some records is an
error. Metadata and object counts alone cannot constitute diagnostic evidence.
Empty collections remain usable only when another collection supplies an actual
metric and no explicit field mapping requires samples from the empty source.

Default aliases are selected per record in declared order; a present invalid
value cannot fall through to a healthy alias. Explicit `*_field` mappings select
exactly that field and require it, rather than silently falling back to default
names. Configured sources/fields/prefixes must be nonempty strings; a prefix
must also retain an ASCII letter or digit after normalization. `null` or omitted
configuration retains defaults. Unconfigured optional third components are not
inferred. Explicit third components are required and included in derived norms.
Magnitude-only records remain valid; absent magnitudes require both configured
planar components, while present selected components must still be numeric.

Streaming reductions retain bounded auxiliary state instead of arrays of
all scalar values and per-record vector allocations. Required sums, means,
spans and norms must be finite; chained `hypot` avoids squaring overflow and
underflow for representable vector norms. Peaks preserve their existing aliases,
signed scalar maximum and last-record tie identity. Magnetic `stored_energy`
retains its legacy whole-group fallback only when density evidence is wholly
absent; it cannot fill missing density rows. This fallback still uses the legacy
output label and is not a dimensional conversion or an independently qualified
energy-density measurement.

The [diagnostic integrity report](../reports/workflow-diagnostic-integrity-20260924.md)
covers three-domain failure isolation, corrected-input replay, visible blocking
for wholly absent quality metrics, and six real triangle/quad solver-to-diagnostic
quality chains. These are local in-process contracts, not remote restart,
large-mesh performance, unit reconciliation, compensated-sum accuracy or general
physics qualification.

The `workflow-thermo-transport-integrity` follow-up extends checked samples and
convergence admission to thermo-mechanical and transport diagnostics. Thermo
retains temperature-delta distributions, displacement norms and its scalar
stress/strain outputs. Default stress aliases are selected per record. A global
`max_stress` is considered only if the whole default element stress group is
absent; selected invalid/partial stress cannot fall through to that summary or
component data. Explicit stress/strain field mappings are mandatory. Default
strain scalars precede component fallback. Every present component axis requires
complete samples; absolute component peaks retain their sign and existing
`x`, `y`, `z`, `xy` ordering. This is not tensor-norm or equivalent-strain
reconstruction. Unselected component aliases and summaries are not reconciled.

Transport no longer invents zero concentration means or source totals/counts
when those groups are absent. Actual zero source samples remain valid; missing
source evidence keeps default quality and composite objectives blocked. Signed
scalar fluxes retain precedence and absolute-magnitude peak ranking. Only absent
scalar flux uses the two-component vector fallback, with both finite components
required. The concentration mean avoids an unused overflowing raw sum; exported
spans and source sums must remain finite. Transport keeps its literal trimmed
output prefix rather than adopting the other domains' prefix normalization.

The [thermo/transport integrity report](../reports/workflow-thermo-transport-integrity-20260925.md)
records boundary controls and three real thermo-triangle, thermo-quad and
transport-bar solver-to-quality chains, including corruption, branch recovery
and corrected-result replay. Shared sample mechanics remain separate from
domain-specific field meanings; no solver equations, task scheduling or SDK
interfaces change. This does not qualify the existing signed-metric scoring
heuristics, arbitrary 3D strain tensors, units, remote recovery or large meshes.

The `workflow-field-bundle-integrity` follow-up covers Rust field statistics,
hotspots, diagnostic-bundle composition and bundle guards. The selected array
must be nonempty and every row must provide a finite selected value. Invalid
explicit field, threshold, percentile, sort or sampling options are errors, not
silent default substitutions. Requested percentiles must be valid and unique
after output-key normalization. Valid threshold precedence, stable tied-sample
ordering and the 32-record sample cap remain unchanged.

Statistics use checked required sums and scaled `hypot` deviations rather than
squaring large or tiny differences. Percentile samples are sorted once, only
when requested. Hotspot means do not require an unused raw sum; sorting borrows
records and only the bounded published samples clone full records. Hotspot IDs
remain an unbounded output list as before. These changes are not a measured
speedup or a general floating-point error bound.

Bundle sources claiming the diagnostic contract must have valid metadata and
actual numeric measurements. Root and selected-source convergence checks run
even if payload retention is disabled. Missing node/element counts become
`null` (unknown), not invented zeros; complete totals use checked integer
addition. Present invalid counts cannot fall through to legacy prefixed counts.
Bundle guards validate every configured rule against an exact finite published
field. Missing sources, missing metrics and invalid rules cannot be counted as
passed checks. Genuine threshold violations still produce a valid blocked or
warning report. On opt-in branch recovery, invalid evidence prevents report
export while raw inputs and independent work remain available.

The [field/bundle integrity report](../reports/workflow-field-bundle-integrity-20260925.md)
records positive controls, corruption/replay and fully prescribed heat/electric
patch results fed through the reporting graph. This does not validate arbitrary
hand-authored report provenance, generic result summaries or CSV exports.
The follow-up `workflow-reporting-cross-runtime-contract` profile now exercises
65 shared JSON cases in both the Rust engine and Elixir Orchestra paths. The
Elixir reporting/summary facades delegate these four operations to checked field
and bundle modules; their obsolete permissive implementations are removed.
Dispatch preserves malformed explicit configuration for validation instead of
turning `false` into defaults. Default p90 hotspot selection, full-precision
percentile keys, null hotspot IDs, deterministic source ordering and unknown
counts now follow the checked Rust contract. BEAM arithmetic overflow becomes a
path-bearing contract error; unrelated exceptions are not swallowed.

The [cross-runtime report](../reports/workflow-reporting-cross-runtime-20260925.md)
also records the shared reporting graph, local TaskIR execution, API rejection
and fresh corrected replay. Comparison uses explicit expected fields and
relative floating-point tolerances, not bitwise whole-output equality. Markdown
preserves unknown counts as `null` instead of blank or invented zero. This is
still in-process evidence: no remote Agent, installed GUI or durable restart is
claimed for that reporting profile. Dedicated domain diagnostics, quality
scoring and other duplicate reducers still require their own cross-runtime
audits. Do not extend that profile to those paths.

The follow-up `workflow-branch-recovery-contract` profile closes the local
Elixir branch-recovery gap. Twenty shared policy cases run against six reporting
faults and two node orders in both runtimes. Exact `skip` is opt-in; absent policy
or explicit `fail` remains fail-fast. A top-level policy takes precedence over
the nested policy, but both declared values must be valid `skip`/`fail` strings.
Malformed policy values are rejected before any operator runs, even if the
node's input would otherwise succeed.

Failed nodes have no published outputs; dependent nodes without their inputs
are skipped, not retried. Independent raw artifacts remain available. A
`transform.first_available` merge waits until its upstream nodes are resolved
and selects the first available source in graph edge order, matching the Rust
topological runner rather than racing a pending preferred input. If no source
survives, it is skipped without inventing a result.

Elixir full, compact and automatic-compact responses preserve `failed_nodes`
and `node_failures` (`node_id`, `kind`, `operator_id`, `error_message`). Progress
tracks successful `completed_nodes` separately from `skipped_nodes` and
`failed_nodes`; `resolved_nodes` is their sum. Async jobs retain the failure
receipts and explicitly warn in the completion message. Job `completed` means
the scheduling run ended, not that every node succeeded or physics were valid;
consumers must inspect these receipts and numerical guard decisions.

The [branch-recovery report](../reports/workflow-branch-recovery-20260925.md)
records direct runner, synchronous API, async persistence, cancellation and
existing application-restart regression checks. It does not claim remote Agent
recovery, installed GUI warning display, arbitrary plugin side-effect rollback,
whole-runtime equivalence or qualification of any additional physics solver.

The `workflow-graph-preflight-contract` follow-up moves structural admission in
front of Elixir callback execution and async job creation. Duplicate node/edge
IDs, ambiguous incoming ports, missing endpoints, artifact-type mismatches and
cycles (including disconnected components) are rejected before progress or
operator callbacks run. A node's `on_error: skip` does not bypass this gate.
Invalid async submissions return an error without leaving job or result rows.

The [graph-preflight report](../reports/workflow-graph-preflight-20260927.md)
retains 51 shared structural cases in both node orders and 26 shared budget
boundary cases. Limits match the tested Rust contract: 2,048 workflow nodes,
4,096 edges, 32 ports per direction, depth 64, 20,000 JSON values per config
and 500,000 per supplied input artifact. These are workflow and wire-data
budgets, not a finite-element mesh-node capacity claim. JSON strings and keys
are byte-bounded and cannot contain NUL. All 71 built-in templates pass the
structural check; missing input values remain an execution/recovery decision.

The Elixir gate is intentionally independent of operator implementations and
allows custom callback operators with valid identifiers. Rust's built-in
capability check, full typed metadata decoding, output-publication budgets,
installed desktop behavior and remote Agent behavior are not covered by this
shared admission claim. Elixir also retains its existing omitted-array
defaults; this is not blanket equivalence with Rust's typed request decoder.
The native validation profile is storage-independent; API checks require a
separate disposable database as described in the report.

The separate `workflow-artifact-publication-contract` profile now closes the
Elixir generated-output gap for solve, transform, extract and export callbacks.
An `{:ok, value}` is checked before any port or lineage is published: at most
500,000 JSON values, depth 64, 500,000 bytes per string and 256 bytes per key;
NUL, invalid UTF-8 and native non-JSON terms are rejected. This validation runs
once per returned value, including nodes without declared output ports, and
does not silently truncate successful data. Malformed callback envelopes
become `invalid_operator_result`; raised exceptions, exits and cancellation
retain their existing handling rather than being swallowed.

The [output-publication report](../reports/workflow-artifact-publication-20260927.md)
records 21 shared Rust/Elixir recipes, injected callback failures, real JSON
export expansion, full/compact responses and async failure persistence. An
explicit skip retains independent raw evidence, publishes no failed outputs
and skips blocked descendants. A completed async job may still contain failed
nodes; its receipts must be inspected. Plain input/condition/output forwarding
keeps the separate input budget, matching Rust, so this is not a universal
500,000-byte response limit. Callback allocation, aggregate retained memory,
external side-effect rollback and remote/installed qualification remain out
of scope. Native improper lists now produce contract errors in the JSON walker
and direct graph preflight instead of crashing those validators.

`solve.cohesive_interface_1d` now has a retained component-level screening
profile for a scalar Mode-I bilinear traction-separation history. The analytic
regression checks onset, peak, linear softening, complete tensile failure, and
the active tangent; cyclic points verify monotonic damage with damaged-secant
unloading and reloading. A separate boundary closes the fully failed interface
in compression and verifies the independent penalty response without healing.
Protocol, Agent RPC, engine workflow, Rust headless discovery, and self-hosted
Web submission tests retain the same public request. This is a standalone
constitutive operator. `solve.cohesive_interface_2d` now reuses that retained
history law in a four-node zero-thickness line interface. Two-point Gauss
integration retains independent directional histories and emits the complete
`8 x 8` material tangent. Closed-form checks cover local opening and shear,
directional softening tangents, rigid coordinate rotation, rigid translation,
the antisymmetric endpoint-jump mode, tangent finite differences, cyclic
history, and exact nodal-force self-balance. The retained
scope is a prescribed single-element response with uncoupled directional
damage. `solve.cohesive_interface_mesh_2d` now assembles those kernels into a
constrained multi-element equilibrium model. Its incremental Newton path
resolves a material catalog, assembles shared-node forces and tangents, reports
nodal reactions, and commits Gauss-point history only after convergence.
Single- and two-element closed forms retain the uniform-opening displacement
and reaction distribution. Optional non-zero constrained displacements advance
proportionally with the load factor, allowing retained monotonic paths to cross
peak traction, follow the softening closed form, and reach complete failure
without a false residual reaction. An alternative explicit control history
accepts an independent load factor and constrained-node displacement vector at
every step. Retained cyclic loading freezes damage on unload and resumes it
beyond the previous peak; a shear-then-opening path proves that directional
histories remain independent. Each step reports displacement, prescribed
displacement, reaction, traction, residual, and directional damage summaries.
Optional two-node linear component connector springs now share the same global
translational DOFs and Newton assembly. A retained series-system closed form
checks that connector force balances the cohesive nodal force, cohesive opening
plus connector extension equals driver displacement, and reported connector
strain energy is exact. Connector displacement, force, energy, and per-step
maximum force remain visible in the result. Optional small-displacement linear
2D host trusses now reuse the public `solve.truss_2d` element and result
contracts. They contribute physical `EA/L` stiffness to the same global system;
a retained length-one series reference verifies exact displacement
decomposition, interface/axial-force balance, strain, stress, force, and energy
density through Solver, Agent RPC, and Engine Workflow. A deliberately
underconstrained rigid mode retains singular-tangent diagnostics and zero-state
rollback. Constant-strain plane-stress host triangles now reuse the public
`solve.plane_triangle_2d` element/result contracts and contribute continuum
stiffness directly to the same Newton system. A prescribed-apex series reference
recovers the analytic interface opening, continuum extension, common force,
strain, stress, and energy through Solver, Agent RPC, and Engine Workflow.
Fully integrated bilinear plane-stress host quads now reuse the public
`solve.plane_quad_2d` element/result contracts in that same assembly. A
rectangular series reference independently recovers interface opening `0.005`,
host extension `0.01`, common force and stress `5`, and exact strain energy;
duplicate IDs, invalid connectivity, and non-positive Gauss-point Jacobians are
rejected before Newton iteration. Linear Euler-Bernoulli 2D host frames now
reuse the public `solve.frame_2d` element/result contracts. Rotational DOFs are
appended after the stable translational layout, and the native transformed
`6 x 6` stiffness contributes directly to the same Newton matrix. A translating
root with a tip-loaded cantilever retains the exact relative deflection
`P L^3 / (3 E I)`, tip rotation `P L^2 / (2 E I)`, root moment `P L`, bending
stress, and strain energy through Solver, Agent RPC, and Engine Workflow.
Unused rotations are constrained automatically; orphan rotational loads and
invalid frame sections fail before iteration.
All cohesive and host kernels now write through the shared sparse
`MatrixAssembler` contract. Free-DOF reduction preserves that sparse shape;
narrow positive-definite tangents use symmetric-band Cholesky with refinement,
while invertible indefinite or wide tangents within the retained bound use a
pivoted dense fallback. A 96-element scale regression retains 384 nodes, 768
DOFs, 3,072 nonzeros, a `0.005208` fill ratio, and an observable
`symmetric_band_cholesky` method through Solver, Agent RPC, and Engine Workflow
result contracts.

`solve.cohesive_interface_mesh_3d` now provides the corresponding retained
three-dimensional surface path rather than projecting the 2D line element into
space. Each six-node triangular interface derives a local orthonormal frame and
uses three-point area integration with two independent tangential histories and
one normal opening/compression history. Optional tetrahedral hosts reuse the
same native element kernel as `solve.solid_tetra_3d` and contribute stiffness,
internal force, stress recovery, and energy to the same global sparse Newton
system. Independent references retain uniform opening, path-dependent unload,
rigid coordinate rotation, and a one-DOF interface/tetra equilibrium split. An
80-element regression reports 1,440 global DOFs, 8,640 nonzeros, fill ratio
`1/240`, and the observable symmetric-band Cholesky path. Protocol, Agent RPC,
Engine Workflow, result chunking, Rust headless discovery, and self-hosted Web
submission retain one public contract. This remains a small-strain triangular
surface with uncoupled directional damage and linear tetra hosts; mixed-mode
coupling, friction, shells, higher-order faces, and scalable sparse-indefinite
factorization remain open.
Protocol, Agent RPC,
engine workflow, result chunking, Rust headless discovery, and self-hosted Web
submission share the same public model. The 2D mesh path remains a 512-node screening solve
under proportional load or displacement control, or explicit prescribed
control history. Global assembly and constraint projection are sparse, but
non-positive-definite or wide reduced tangents still use a dense fallback up to
1,536 free DOFs. Linear 2D trusses and frames are retained structural hosts;
plane-stress triangles and quads are its retained continuum hosts. Shells and
3D solids are intentionally not mixed into that 2D operator;
scalable sparse-indefinite solves, fill-reducing ordering,
arc-length/adaptive continuation, coupled mixed-mode
interaction, frictional delamination, and experimental calibration remain
outside the retained claim.

These four cohesive operators intentionally remain in the component validation
profile rather than the release-gated reliability manifest. That manifest only
accepts physics-coverage operators with retained qualification evidence; adding
a new ad hoc `screening` coverage level would weaken the gate instead of
describing the current claim honestly.
Their separate `cohesive-interface` benchmark matrix now exercises all four
native Engine and workflow paths with bounded constitutive histories and the
retained 2D/3D sparse shapes. It supplies repeatable performance visibility,
but does not promote these component claims into release qualification.

`solve.electric_conduction_plane_quad_2d` now has an explicit component
validation profile rather than relying on scattered implementation tests. The
profile retains a rotated Ohmic rectangle closed form, four mesh-refinement
levels, current and power conservation, malformed quad rejection, Agent RPC,
Engine Workflow, and Rust headless discovery. Its quad preflight rejects
repeated connectivity, degenerate triangles, and inconsistent split-triangle
orientation before assembly. This is screening evidence only; nonlinear,
anisotropic, transient, induction/skin-effect, and coupled thermal feedback
claims remain outside the operator, and release promotion still requires
versioned provenance plus reviewer-approved retained evidence.

The conduction profile also retains common-potential shifts of `+/-2^40 V`
for current-driven, contact, finite-impedance, and mixed-boundary models.
Assembly shifts terminal external voltages together with prescribed values;
field/current recovery and balanced network work use the relative solution.
Fixed-electrode net injection is recovered from voltage differences rather
than cancellation of a large current source and its reaction. Reported nodal
potentials and individual terminal powers retain the caller's absolute
reference. A 1,225-node current-driven patch covers all three sparse
preconditioners, and an Engine test explicitly transfers Joule power into
nodal thermal loads. This is not automatic volumetric projection, nonlinear
electrothermal feedback, a large-mesh benchmark, or broader qualification.
See the [bounded regression report](../reports/electric-conduction-reference-20260920.md)
for reproduction commands, tolerances, and floating-point limits.

The Rust study-level heat projectors now carry the integrated-energy contract
through to dielectric heating: spatial `E_rms^2 = 2 * energy_density / epsilon`,
not the squared mean field vector. Joule projections check finite, non-negative
source powers and actual per-node added loads, rejecting swallowed increments
instead of allowing a `NaN` or tiny-power comparison to pass. Region and total
power accounting use measured increments rather than unconditional zero errors.
Unmapped contacts and finite-impedance terminals both fail explicitly. Equal
four-node power lumping is retained; this is not subcell source quadrature or
new interface heat-mapping support. Dielectric distribution replaces seed loads,
then Joule heating adds to them. The
[projection regression report](../reports/composite-heat-projection-20260920.md)
records the local SDK/solver evidence without changing release qualifications.

The separate Rust study-level electrothermal feedback loop also checks scale-
independent relative changes, not absolute differences below machine epsilon.
Its trace assessment recomputes temperature residuals and successive combined
loss/per-region conductivity changes before accepting convergence. A unit-square
Ohmic-to-heat fixed point is checked at thickness scales `1e-20`, `1`, and
`1e20`, including insufficient iteration budgets, invalid budgets, and clean
retry after material feedback failure. Both heating mechanisms and temperature-
dependent thermal conductivity are exercised together as well. This bounded
study-loop evidence does not extend the linear conduction operator's physical
scope or establish general nonlinear stability. See the
[feedback convergence report](../reports/composite-feedback-convergence-20260923.md).

The subsequent heat-to-structural handoff uses a shared one-to-one nodal map
instead of repeated linear searches or raw index equality. Source records must
agree with their retained heat mesh, and reordered structural nodes/elements
must retain IDs, coordinates and cyclic connectivity. Empty/ambiguous fields,
nonfinite or overflowed temperature differences, and unrepresentable expansion
coefficients fail explicitly. Real-solver tests retain restrained stress,
stress-free expansion, nonuniform temperature transfer and shared-node regional
materials. Unsupported negative expansion coefficients are rejected before
structural dispatch; the solver's physical domain is not broadened. See the
[thermal projection report](../reports/composite-thermal-projection-20260923.md)
for bounds, a local mapping microbenchmark, and reproducible commands.

The layered composite heat reference now judges the predicted temperature rise,
not the ambient-inclusive Celsius value. Regional source assembly rejects
overflow, underflow and lost shared-node increments rather than accepting a
small absolute whole-model error. Real heat solves check three conductivity
sets, three regional source patterns and four mesh levels against an independent
Fourier-law integration at every node and element. Shared-face flux, source
jumps and total outlet heat are checked separately; scaling conductivity and
source power together preserves temperature and scales flux. This evidence is
limited to the three-layer ideal shared-node fixture. Contact thermal resistance,
interface temperature discontinuities, arbitrary geometries and nonlinear
material qualification are not covered. See the
[layered heat validation report](../reports/composite-heat-interface-validation-20260923.md).

Finite contact resistance is a separate, locally screened extension of the
planar heat operators. Matching boundary edges use consistent linear interface
integration, with independent temperatures and equal/opposite transfer. The
new `heat-plane-contact-screening` profile tests series resistance, nonuniform
free interface temperatures, edge refinement, shifted/rotated geometry, malformed
contacts and the headless-to-engine-to-structural chain. It does not extend the
historical ideal-interface release qualification. See the
[operator contract](thermal-contact-operator.md) and
[contact regression report](../reports/heat-plane-contact-20260923.md).

`solve.frame_3d` is now qualified for the current single-member cantilever
scope. The retained evidence derives the Euler-Bernoulli displacement, slope,
root moment, bending stress, and strain-energy formulas for an x-aligned 3D
frame, then checks them against
`workers/rust/crates/solver/tests/frame_3d_closed_form.rs`. The retained
scaling regression verifies that tip-load changes linearly scale displacement,
rotation, root moment, and bending stress while scaling strain energy
quadratically, and that bending-inertia changes inversely scale displacement,
rotation, and energy without changing load-controlled moment or stress. It
also verifies length scaling across element length, displacement, rotation,
root moment, bending stress, and strain energy for the same tip-load case.
Every retained branch also re-derives displacement, rotation, moment, stress,
energy summary fields, node id/coordinate passthrough, element length, axial
stress, bending stress, and combined stress from nodes/elements and checks
`total_strain_energy = 0.5 * sum(load_or_moment * displacement_or_rotation)`.
This remains a single-member linear static qualification, not a multi-member
stability, geometric nonlinearity, warping, plastic-hinge, or dynamics claim.

`solve.plane_triangle_2d` and `solve.plane_quad_2d` are now qualified for the
current small plane-stress patch scope. The retained evidence checks the
triangle direct-stiffness reference, the quad split-triangle weighted contract,
stress diagnostics, von Mises handling, and strain-energy totals against
`workers/rust/crates/solver/tests/plane_2d_closed_form.rs`. The retained
scaling regressions verify that load changes scale displacement, stress, and
energy by the expected linear/quadratic factors, that triangle and quad
thickness changes inversely scale displacement, stress, and energy under fixed
nodal loads, and that triangle and quad modulus changes inversely scale
displacement and energy without changing load-controlled stress. Both retained
paths also check similar-geometry scaling where area scales quadratically,
stress scales inversely with length, and displacement/energy stay fixed under
the same nodal loads. They also assert the global work-energy conjugacy
`total_strain_energy = 0.5 * sum(load_x * ux + load_y * uy)` for both retained
triangle and quad patch paths. Every retained branch also re-derives node
id/coordinate passthrough, displacement magnitude, triangle/quad element area
from node coordinates, max displacement, max stress, max strain-energy density,
total strain energy, and work-energy consistency from public result fields.
Triangle elements additionally recheck the direct principal-stress,
von Mises, in-plane shear, and `0.5 * stress dot strain` energy-density
formulas; quad elements keep those nonlinear diagnostics as split-triangle
weighted result fields rather than incorrectly reapplying the formula to
weighted stress/strain components. This remains a small-patch
qualification, not a mesh-convergence, high-order quadrature, distorted-element,
plasticity, buckling, or large-deformation claim.

The `beam-frame-classic` qualification candidate is now approved for
qualification. Its reference note is
`evidence/operator-qualification/beam-frame-classic-reference-note.md`, and its
first multi-case regression is
`workers/rust/crates/solver/tests/beam_frame_classic_regression.rs`. That test
checks a closed-form cantilever beam, equivalent 2D frame cantilever, and
prismatic torsion shaft. The retained regression now also checks beam tip-load
and bending-inertia scaling, beam length scaling, 2D frame tip-load and
bending-inertia scaling, plus torsion torque, polar-moment, and length scaling,
so load-controlled moments, torques, stresses, and strain energies stay
separated from stiffness- or geometry-controlled displacement, rotation, or
twist response. It also checks the signed work-energy conjugacy
`total_strain_energy = 0.5 * tip_load * tip_displacement` for beam and frame
cases, and `total_strain_energy = 0.5 * torque * twist` for torsion. Every
retained branch also re-derives displacement, rotation, moment, torque, stress,
and total strain-energy summaries from node and element fields. The torsion
branch additionally re-derives element length, twist angle, torque, shear stress,
and element strain energy from public node and input fields. The beam and frame
branches re-derive element length and bending stress; the frame branch also
re-derives axial stress and combined stress from public node, element, and input
fields. Its sign convention note is
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

## Condition Decision Reliability

The `workflow-condition-contract` profile compares Rust and Elixir against 64
shared condition cases, in both node orders, with fail-fast and explicit skip
controls. It also enumerates bounded exact numeric oracles and retains local
HTTP/SQLite failure-recovery tests. Invalid array indexes cannot select the last
element; malformed configuration becomes a node error; integer comparisons do
not collapse through `f64` rounding. JSON equality and array containment use the
same recursive numeric equivalence, without conflating booleans and numbers.

Missing paths still yield null, and missing operators default to `gt`; explicit
`falsy`/`eq null` remains allowed. This is neither a physical-tolerance policy
nor arbitrary-precision decimal support. Failures retain their diagnostic and
publish no branch outputs; only explicit skip permits independent work to
continue. Unrelated callback exceptions and cancellation are not swallowed.

Run `make test-workflow-condition-contract` for storage-independent Elixir
regression or use the native validation profile for Rust/Elixir checks together.
The [condition reliability report](../reports/workflow-condition-contract-20260927.md)
records the scoped results and isolated-database API procedure. This adds local
`verified` tensor evidence, not distributed or numerical qualification.

## Named Input Reliability

The `workflow-named-input-contract` profile exercises nine domain benchmark-pair
operators and `transform.join_parameter_sweep_results` through both Rust and
Elixir graphs, rather than testing only their direct operator calls. Named inputs
are assembled by target port, independent of edge or node order. A single
declared `input` or `payload` port accepts an already assembled operator payload;
one remaining edge on a multi-port node is not an envelope fallback.

For these operators, a legacy artifact containing only its source-port key is
unwrapped once. Multi-field artifacts retain all sibling data and status fields.
The tests cover raw-result retention, same-source fan-out, failed second inputs,
clean replay, HTTP invocation, and persisted asynchronous receipts. Existing
input behavior for other operators and custom callbacks is unchanged.

Run `make test-workflow-named-input-contract` for storage-independent regression.
See the [routing reliability report](../reports/workflow-named-input-contract-20260927.md)
for isolated-SQLite API tests and the accompanying deleted-job progress race fix.
This is local contract/recovery evidence, not numerical solver qualification.

## Concurrent Job Receipts

Progress, worker, and metadata SQL writes now compare the complete validated
snapshot atomically. Conflicts allow at most four reread/revalidation attempts;
implicit event timestamps are not refreshed, and terminal replays do not write.
The watchdog uses `Store.apply_progress_if_current/2` without rebasing: a newer
heartbeat, terminal receipt, or execution phase invalidates its old decision.
Only actual watchdog writes count as stalls/timeouts; deleted jobs do not stop
the scan. The memory backend implements the same conditional snapshot contract.

The [job snapshot report](../reports/job-snapshot-reliability-20260927.md) records
deterministic races, concurrent writers, isolated backend tests, and timestamp
boundaries. This is local contract/recovery evidence, not PostgreSQL, remote
recovery, persistent incarnation fencing, or physical solver qualification.

## Solver Completion Receipts

For asynchronous solver submissions, Agent progress is not a final job receipt:
`completed` progress means postprocessing, while the final RPC response and
result storage determine completion. Missing/non-object results fail explicitly
instead of stranding the background job. The
[solver completion report](../reports/solver-completion-reliability-20260927.md)
records the initial local HTTP/TCP regressions. Final solver publication now uses
`Store.complete_with_result/3`: worker, result, and completed state commit together,
and an existing terminal job or result cannot be overwritten by a late response.
SQL uses one transaction; the lightweight memory backend uses one digest-verified
`analysis-state.json` generation for both collections, importing legacy files
only on its first start. See the
[atomic publication report](../reports/solver-atomic-publication-20260927.md)
for rollback, process-loss, concurrent-writer, migration, and recovery evidence.
This solver evidence does not qualify external checkpoint transactions,
PostgreSQL, host power loss, exactly-once computation, or physical correctness.

## Atomic Workflow State Commits

`Store.apply_progress_with_result/4` conditionally commits the job and its
existing workflow runtime/recovery record together. Progress, completion,
failure, cancellation, recovery blocking, and claim publication use this boundary;
stale job/result snapshots cannot be rebased or partly published. SQL joins the
owning lease transaction, and memory persists one combined generation. Execution
claims must match generation, attempt, and owner. Cancellation propagates commit
errors, while rejected recovery-block writes are not counted as durable blocks.
The [workflow commit report](../reports/workflow-atomic-commit-20260927.md) records
rollback, stale-claim, process-loss, filesystem-fault, and retry tests. This covers
local runtime records, not external solver checkpoints, live PostgreSQL,
distributed failover, exactly-once execution, or numerical qualification.

The [storage-outage report](../reports/storage-outage-recovery-20260927.md) adds local scan containment, policy-checked retry, and degraded health with explicitly unknown counts.
The [admission report](../reports/workflow-admission-reliability-20260927.md) covers atomic creation, insert-once initialization, and recovery.
The [deletion report](../reports/workflow-deletion-reliability-20260927.md) covers atomic job/result removal and post-commit local runner shutdown.
The [result administration report](../reports/result-administration-reliability-20260927.md) covers lease-guarded, atomic edits/deletion of existing results without recovery-metadata injection or snapshot races.

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

- deepen qualification evidence with larger mesh, boundary, material,
  convergence, or literature-backed references where current scope is still a
  compact retained fixture
- use the approved release packets as templates for new physics families before
  they enter the release-gated `physics-coverage` manifest
- keep new experimental operators outside the release gate until they have a
  qualification path, rather than weakening the moxi coverage contract
- keep Stokes-flow qualification scoped to the retained screening-boundary
  convergence fixture until a stronger CFD benchmark or reference-tool
  comparison exists
- keep future qualification promotions blocked until external, convergence,
  literature, or analytic evidence exists
