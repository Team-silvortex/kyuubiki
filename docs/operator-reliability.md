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
roadmap candidates reference existing manifest operators, already satisfy the
roadmap's minimum candidate level, and never set that candidate minimum below
the manifest's release-gated `minimum_coverage_level`.
Each roadmap candidate also carries a machine-readable qualification posture:

- `target_level`
  the trust level the candidate is trying to reach. Release-gated roadmap
  candidates for `moxi 2.0.x` target `qualification`; screening-only or
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
The retained mesh-refinement regression uses a linear Stokes field on 1x1,
2x2, 4x4, and 8x8 quad/triangle meshes to verify stable area, divergence, shear,
velocity, pressure-drop, viscous-stress, and total viscous-dissipation
diagnostics. That is enough to
qualify the Stokes screening boundary, but not enough to claim general CFD,
Navier-Stokes, turbulence, compressible-flow, or industrial design accuracy.
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
summary maxima, pressure drop, element average velocity/pressure, Reynolds
number, and nonnegative divergence, shear, stress, and dissipation diagnostics
from node and element fields.

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
The retained thermoelastic patch also checks temperature-delta and thickness
scaling: restrained thermal stress scales linearly with temperature delta,
strain-energy density and total energy scale quadratically with temperature
delta, and thickness leaves stress plus energy density fixed while scaling
total energy linearly. Its retained regression also re-derives displacement,
temperature-delta, von Mises stress, in-plane shear, thermal strain,
element geometry area, strain-energy density, and total strain-energy summaries
from public result fields.
An independent triangle/quad refinement regression now applies the same fully
restrained uniform temperature rise on 1x1, 2x2, 4x4, and 8x8 meshes. It keeps
zero displacement, thermal strain, peak stress, energy density, and total
strain energy invariant across both discretizations. This is a uniform-field
thermoelastic proof point, not a claim for arbitrary coupled thermal boundary
conditions.
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
for shorter beams across eigenvalue, rad/s, Hz, and period. The retained
regression also verifies that uniform stiffness and density scaling drive modal
frequency by `sqrt(stiffness / density)`, eigenvalue by `stiffness / density`,
and period by the inverse frequency factor while preserving free DOFs and
normalized participation. Every retained branch also re-derives min/max
frequency, total mass from element density, area, and node-coordinate length,
eigenvalue/rad/s/Hz/period consistency, mode index order, expanded shape
constraints, and participation norm from the retained mode fields.
Symmetric 3D bending
modes may be near-degenerate, so the 3D ordering contract is non-decreasing
rather than strictly increasing.
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
contiguous history step numbering, and final spring/damping force diagnostics.
The harmonic branch checks dynamic-stiffness amplitudes, damping response,
retained input frequency order, harmonic node id passthrough, fixed-node
zero-amplitude phase, global maxima, peak frequency,
per-frequency maxima, and velocity/acceleration amplitudes from the
frequency result fields.

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
step, contiguous step numbering, final time, final node temperatures, element
length from input coordinates, final heat flux, and thermal energy.
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
scaling. Every retained branch also re-derives summary maxima, sound-pressure
level, speed of sound, wave number, particle velocity, acoustic intensity,
damping loss, element length, node coordinate/source passthrough, and material
echo fields from node, element, and material fields. This does not claim
branched duct networks, nonlinear acoustics, transient propagation, or 3D
acoustic cavities.
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

For continuous, index-adjacent 1D chains, the heat, electrostatic,
magnetostatic, advection-diffusion, and acoustic bar solvers use a constrained
tridiagonal direct path. Branched, reordered, or otherwise non-chain inputs
remain on their established sparse or dense fallback paths. This keeps the
specialization topology-scoped while making million-node chain studies bounded
by linear storage and solve work rather than iterative convergence.

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
time-varying fields.

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
This does not claim nonlinear
springs, transient dynamics, contact, or arbitrary vector spring networks.

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
This does not claim nonlinear springs, contact, transient dynamics, or general
mesh-convergence behavior for arbitrary spring networks.

The thermal beam 1D operator is now qualified for the retained linear
free-curvature scope. Its closed-form evidence checks thermal curvature, tip
rotation, tip displacement, zero-gradient response, and near-zero internal
force for a single fixed-free member. The retained scaling regression verifies
that temperature-gradient and thermal-expansion changes linearly scale
curvature, tip rotation, and tip displacement, while section-depth changes
inversely scale those same free-curvature diagnostics. It also verifies length
scaling for the same free-curvature case: curvature stays fixed, tip rotation
scales linearly, and tip displacement scales quadratically, without
introducing internal moment or strain energy. Every retained branch also checks
that total strain energy equals the sum of member `strain_energy` diagnostics,
so free-curvature zero-energy behavior is enforced at both summary and element
levels. Every retained branch also re-derives node displacement magnitude,
node id/coordinate passthrough, max displacement, max rotation, max moment, max
stress, max temperature gradient, thermal curvature, and total strain energy
from public node and element fields.
This does not claim thermal frame assemblies, nonlinear material behavior,
transient heat transfer, buckling, or plasticity.

The thermal truss 3D operator is now qualified for the retained fully
restrained uniform-temperature scope. Its closed-form evidence checks zero
node displacement, thermal/mechanical strain split, compressive stress, axial
force, and member energy summation. The retained scaling regression verifies
that temperature and thermal-expansion changes scale thermal strain, stress,
and axial force while scaling strain energy quadratically, that Young's
modulus changes scale stress, axial force, and energy without changing thermal
strain, and that area changes scale axial force and total energy without
changing stress or energy density. It also verifies uniform geometry scaling
where member lengths and total energy scale linearly while thermal strain,
stress, axial force, and energy density remain fixed. Every retained scaling
branch also re-sums member energy from
`strain_energy_density * area * length` and checks max energy density against
the element set. Every retained branch also re-derives average element
temperature delta, node id/coordinate/temperature passthrough, thermal strain,
mechanical strain, Hooke-law stress, axial force, max displacement, max
temperature delta, max stress, max axial force, and total strain energy from
public node and element fields. This does not claim partial restraint,
temperature gradients, buckling, plasticity, contact, or dynamic response.

The thermal truss 2D operator is now qualified for the retained fully
restrained uniform-temperature scope. Its closed-form evidence mirrors the 3D
thermal truss lane by checking fixed node displacement, thermal/mechanical
strain split, compressive stress, axial force, and member energy summation.
The retained 2D scaling regression applies the same temperature,
thermal-expansion, Young's-modulus, area, and uniform geometry checks to the
planar restrained triangle, including total member-energy re-summation and max
energy-density checks across every retained scaling branch. Every retained
branch also re-derives average element temperature delta, node
id/coordinate/temperature passthrough, thermal strain, mechanical strain,
Hooke-law stress, axial force, max displacement, max temperature delta, max
stress, max axial force, and total strain energy from public node and element
fields. This does not claim partial restraint, mixed thermal loading,
temperature gradients, buckling, plasticity, contact, or dynamic response.

The thermal frame 2D operator is now qualified for the retained fully
restrained uniform-temperature single-member scope. Its closed-form evidence
checks fixed end displacement and rotation, thermal/mechanical strain split,
axial force, axial stress, zero-gradient moment and shear response, and strain
energy. The retained scaling regression verifies that temperature changes
and thermal-expansion changes scale thermal strain, axial force, and axial
stress while scaling strain energy quadratically, that area changes scale
axial force and energy without changing stress, and that modulus changes scale
force, stress, and energy without changing thermal strain. It also verifies
length scaling where thermal strain, stress, and axial force remain fixed while
strain energy scales linearly with member length. Every retained branch also
checks that total strain energy equals the sum of member `strain_energy`
diagnostics. Every retained branch also re-derives node displacement magnitude,
node id/coordinate/temperature passthrough, max displacement, max rotation,
average temperature delta, thermal strain, mechanical strain, thermal
curvature, axial stress, axial force, combined stress, max temperature delta,
max temperature gradient, max moment, max stress, and total strain energy from
public node and element fields. This does not claim partial restraint,
temperature gradients, frame assemblies, geometric nonlinearity, buckling,
plasticity, contact, or dynamic response.

The thermal frame 3D operator is now qualified for the retained fully
restrained single-member scope with uniform temperature and linear gradients.
Its closed-form evidence checks fixed end translations and rotations,
thermal/mechanical strain split, thermal curvatures, axial force, bending
moments, combined stress, and strain energy. The retained scaling regression
verifies that uniform temperature and gradient scaling drives thermal strain,
thermal curvatures, axial force, bending moments, and energy by the expected
linear/quadratic factors, thermal-expansion scaling drives the same retained
thermal strain, curvature, force, moment, and energy factors, Young's modulus
scales force, moment, and energy without changing thermal strain or curvature,
and inertia scaling changes bending moments without changing thermal curvature
or axial force. It also verifies length scaling where thermal strain,
curvatures, axial force, and bending moments remain fixed while strain energy
scales linearly with member length. Every retained branch also checks that
total strain energy equals the sum of member `strain_energy` diagnostics. Every
retained branch also re-derives node displacement and rotation magnitudes, max
displacement, node id/coordinate/temperature passthrough, max rotation, average
temperature delta, thermal strain, mechanical strain, both thermal curvatures,
axial stress, axial force, bending moments, bending stress, combined stress,
temperature summaries, moment summary, stress summary, and total strain energy
from public node and element
fields. This does not claim partial restraint,
arbitrary 3D frame
assemblies, torsion-dominant response, geometric nonlinearity, buckling,
plasticity, contact, or dynamic response.

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
public result fields. This does not claim multidimensional contact, friction,
impact, large deformation, or industrial contact search.

The truss 2D operator is now qualified for the retained symmetric two-bar
scope. Its closed-form evidence checks fixed supports, apex symmetry, vertical
displacement, equal axial member force, stress, strain, and strain energy.
The retained scaling regression also verifies that load changes linearly scale
apex displacement, axial force, and stress while scaling energy quadratically,
and that area changes inversely scale displacement, stress, and energy while
preserving axial force. It also verifies that Young's modulus changes
inversely scale displacement and strain energy while preserving
load-controlled axial force and stress, and that similar-geometry scaling
linearly scales member length, apex displacement, and strain energy while
preserving axial force and stress. It also asserts
`total_strain_energy = 0.5 * apex_load * apex_displacement` across the retained
closed-form and scaling cases. Every retained branch also re-derives maximum
nodal displacement, node id/coordinate passthrough, max stress, max
strain-energy density, total strain energy, Hooke-law stress, axial force,
element energy density, and global external work from public node and element
fields. This does not claim arbitrary truss topology, geometric nonlinearity,
buckling, dynamic response, damaged members, or 3D space truss behavior.

The truss 3D operator is now qualified for the retained symmetric tripod scope.
Its closed-form evidence checks fixed base supports, zero lateral apex motion,
vertical displacement, equal axial leg force, stress, strain, and strain
energy. The retained tripod scaling regression applies the same load and area
checks to the 3D leg-force/stress/energy response, and now also verifies
Young's-modulus inverse displacement and energy scaling while preserving
load-controlled leg force and stress. It also checks similar-geometry scaling
across leg length, apex displacement, and strain energy while preserving leg
force and stress, plus
`total_strain_energy = 0.5 * apex_load * apex_displacement` for the retained
vertical load/displacement pair. Every retained branch also re-derives maximum
3D nodal displacement, node id/coordinate passthrough, max stress, max
strain-energy density, total strain energy, Hooke-law stress, axial force,
element energy density, and global external work from public node and element
fields. This does not claim arbitrary space-frame topology, geometric
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
public node and element fields. This remains a single-element linear-elastic
qualification, not a mesh-convergence,
plasticity, contact, or large-deformation claim.

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
fields. This is a monotone
one-dimensional hardening qualification, not a hysteresis, softening,
snap-through, or dynamics claim.

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
