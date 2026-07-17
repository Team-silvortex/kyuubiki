# Accuracy Baselines: v1.x Seed Set

This is the first concrete baseline set for `v1.x` numerical trust work.

These cases are intentionally small, hand-checkable, and fast. They exist to
turn the accuracy plan into something that will fail loudly if we regress core
mechanics.

## Current seed set

| Study | Case | Reference metrics | Tolerance | Automation |
| --- | --- | --- | --- | --- |
| `axial_bar_1d` | one-element tensile bar | tip displacement `4.761904761904762e-7`, max stress `100000`, reaction `-1000` | absolute tolerances from solver unit baseline | automated |
| `thermal_bar_1d` | restrained uniform temperature rise | max displacement `0`, max stress magnitude `100800000`, max axial force magnitude `1008000`, max temperature delta `40` | relative tolerance `1e-9` on force/stress magnitudes | automated |
| `spring_1d` | one-dimensional spring chain sample fixture | max displacement `0.09428571428571428`, max force `1200`, node displacements `(0.03428571428571429, 0.09428571428571428)`, element forces `(1200, 1199.9999999999998)` | absolute tolerances from solver unit baseline | automated |
| `spring_2d` | planar spring grid sample fixture | max displacement `0.06339734949589224`, max force `1120.754716981132`, node-2 `(ux, uy)=(0.0509433962264151, -0.03773584905660377)`, element forces `(1120.754716981132, -679.2452830188679, 112.06975399937737)` | absolute tolerances from solver unit baseline | automated |
| `spring_3d` | spatial spring cage sample fixture | max displacement `0.05955868626521211`, max force `803.0108273796119`, top-node `(ux, uy, uz)=(0.037134189113355795, 0.03445543981481482, -0.03132270383761861)`, loaded member forces `(-803.0108273796119, -474.11760144504234, -82.59674462242567)` | absolute tolerances from solver unit baseline | automated |
| `thermal_beam_1d` | free gradient beam sample fixture | max displacement `0.005184000000000001`, max rotation `0.004320000000000001`, max moment `7.275957614183426e-12`, max stress `6.614506921984932e-9`, max temperature gradient `45` | absolute tolerances from solver unit baseline | automated |
| `heat_bar_1d` | two-element conduction gradient with fixed end temperatures | max temperature `100`, max heat flux `1800`, middle-node temperature `60`, element gradient `-40` | absolute tolerances from solver unit baseline | automated |
| `heat_plane_triangle_2d` | two-triangle conduction patch with three fixed temperatures | max temperature `100`, max heat flux `3600`, node-1 temperature `60`, element gradients `(-40, -40)` and `(0, -80)` | absolute tolerances from solver unit baseline | automated |
| `heat_plane_quad_2d` | single quad conduction patch with three fixed temperatures | max temperature `100`, max heat flux `2846.0498941515416`, node-1 temperature `60`, gradient `(-20, -60)` | absolute tolerances from solver unit baseline | automated |
| `beam_1d` | tip-loaded cantilever beam | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `torsion_1d` | torsion shaft sample fixture | max rotation `0.026371308016877638`, max torque `2500`, max stress `20833333.333333332`, tip rotation `0.026371308016877638` | absolute tolerances from solver unit baseline | automated |
| `frame_2d` | tip-loaded cantilever frame member | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `frame_3d` | tip-loaded cantilever space-frame member | max displacement `0.0015873015873015873`, max rotation `0.0011904761904761906`, max moment `2000`, max stress `1.25e7` | absolute tolerances from solver unit baseline | automated |
| `truss_2d` | three-bar triangular truss | max displacement `1.114463950892853e-6`, max stress `60092.52125773316`, tip `ux=2.380952380952381e-7`, tip `uy=-1.088733463909362e-6` | absolute tolerances from solver unit baseline | automated |
| `truss_3d` | space-frame pyramid sample fixture | max displacement `0.0000015799074540869988`, max stress `74386.37868140468`, top-node `(ux, uy, uz)=(2.897530666749509e-7, 2.897530666749509e-7, -0.0000015258420246488773)`, stressed members `(-74386.37868140468, -63387.6959669619, -63387.6959669619)` | absolute tolerances from solver unit baseline | automated |
| `plane_triangle_2d` | two-triangle square patch | max displacement `1.504347441414315e-6`, max stress `100000`, node-2 `ux=4.714285714285715e-7`, node-2 `uy=-1.428571428571429e-6` | absolute tolerances from solver unit baseline | automated |
| `plane_quad_2d` | single quad plate patch sample fixture | max displacement `5.333507749004975e-7`, max stress `126981.38527836032`, node-2 `(ux, uy)=(2.576145151695419e-7, -4.6700943316053366e-7)`, stress `(12500, -120000)`, shear `3048.7804878048746` | absolute tolerances from solver unit baseline | automated |
| `thermal_plane_triangle_2d` | fully restrained two-triangle thermoelastic patch | max displacement `0`, max stress `50149253.731343284`, max temperature delta `40`, element stress `-50149253.731343284` | absolute tolerances from solver unit baseline | automated |
| `thermal_plane_quad_2d` | fully restrained quad thermoelastic patch | max displacement `0`, max stress `34477611.940298505`, max temperature delta `30`, element stress `-34477611.940298505`, mechanical strain `-3.3e-4` | absolute tolerances from solver unit baseline | automated |
| `thermal_frame_2d` | heated portal frame sample fixture | max displacement `0.0010408174194986581`, max rotation `0.0006805479452054797`, max axial force `24164.383561644005`, max moment `42915.94520547945`, max stress `36971506.84931508`, max temperature delta `35`, max temperature gradient `30` | absolute tolerances from solver unit baseline | automated |
| `thermal_truss_2d` | heated triangular truss sample fixture | max displacement `4.801785714285713e-4`, max axial force `235.84952830143558`, max stress `23584.952830143557`, max temperature delta `40` | absolute tolerances from solver unit baseline | automated |
| `thermal_truss_3d` | restrained uniform temperature rise in a 3D truss member | max displacement `0`, max stress magnitude `100800000`, max axial force magnitude `1008000`, max temperature delta `40` | relative tolerance `1e-9` on force/stress magnitudes | automated |
| `thermal_frame_3d` | restrained thermal space frame with dual gradients | max displacement `0`, max axial force `1.764e6`, max moment `2688`, max stress `1.239e8`, max temperature delta `35`, max temperature gradient `30` | relative tolerance `1e-9` on force/moment/stress magnitudes | automated |

## Mechanical convergence and perturbation checks

The first post-baseline mechanical trust ladder is automated in
[mechanical_convergence.rs](../workers/rust/crates/solver/tests/mechanical_convergence.rs).
It currently covers `axial_bar_1d` with:

- mesh refinement invariance across 1, 2, 4, 8, 16, and 32 elements against
  the closed-form tip displacement, stress, reaction, and strain energy
- load perturbation scaling for displacement, stress, and energy
- area perturbation scaling for displacement, stress, and energy
- length perturbation scaling for displacement and energy while preserving
  stress and axial force
- result-field recovery of tip displacement, reaction force, element strain,
  stress, axial force, strain-energy density, and summary maxima from public
  node/element fields

`spring_1d`, `spring_2d`, and `spring_3d` add foundational discrete stiffness
checks. They cover series equivalent stiffness and orthogonal vector spring
closed forms under load and stiffness perturbations, including direction-cosine
assembly, element force/extension recovery, strain-energy accumulation, and
global work-energy balance.

`thermal_bar_1d` adds the first focused thermo-mechanical line-field scaling
ladder. It covers the fully restrained uniform-temperature-rise scope across
temperature, thermal-expansion, modulus, area, and length perturbations, while
re-deriving element length, average temperature delta, thermal strain,
mechanical strain, total strain, stress, axial force, energy density, total
strain energy, and summary maxima from public node, element, and input fields.

It also covers `beam_1d` with tip-loaded cantilever refinement invariance
across 1, 2, 4, 8, and 16 elements against closed-form tip displacement, tip
rotation, root moment, bending stress, and strain energy.

`frame_2d` mirrors the same tip-loaded cantilever closed-form ladder across 1,
2, 4, 8, and 16 elements, but through the 2D frame node DOFs and local/global
frame stiffness path. It checks axial drift stays zero while transverse
displacement, rotation, moment, stress, and strain energy stay closed-form.

`frame_3d` adds the matching space-frame cantilever ladder across 1, 2, 4, 8,
and 16 elements. It checks the six-DOF node path, local/global 3D frame
transform, transverse displacement, rotation, root moment, bending stress, and
strain energy, plus load, inertia, and section-modulus perturbation scaling.

`modal_frame_2d` and `modal_frame_3d` add the first dynamic reliability checks.
They keep the retained cantilever modal contract while verifying total mass,
eigenvalue, natural-frequency, period, and shape contracts under global
stiffness and density scaling, plus short-versus-long cantilever monotonicity
across eigenvalue, rad/s, Hz, and period. Every retained branch also re-derives
min/max frequency, eigenvalue/rad/s/Hz/period consistency, mode index order,
expanded shape constraints, and participation norm from the retained mode
fields.

`transient_spring_1d` and `harmonic_spring_1d` add the first direct dynamic
spring checks. The transient path verifies one-step Newmark average
acceleration updates against a hand-derived single-DOF reference; the harmonic
path checks single-DOF dynamic-stiffness amplitudes, velocity/acceleration
amplitudes, element force amplitude, and peak-frequency selection. The focused
`dynamic_spring_closed_form.rs` regression keeps the same closed-form references
as a promotion-ready evidence entry and adds load-scaling checks: transient
displacement, velocity, acceleration, spring/damping force, kinetic energy, and
strain energy scale by the expected linear/quadratic factors when load and
initial state are scaled together, while harmonic amplitudes scale linearly,
the peak frequency remains unchanged, and increased damping lowers the retained
near-resonance displacement while keeping force amplitude closed-form. It also
checks undamped free-vibration time-step refinement against the continuous
single-DOF oscillator reference, requiring smaller time steps to reduce
displacement error over the retained short transient window. Every retained
branch also re-derives transient history maxima, kinetic energy, strain energy,
final element force diagnostics, harmonic frequency maxima, global maxima, peak
frequency, and velocity/acceleration amplitudes from node and element fields.

`transient_heat_bar_1d` adds a focused heat-transfer transient check. The
`transient_heat_bar_closed_form.rs` regression verifies the implicit Euler
single-free-node thermal recurrence for a lumped-capacity bar with positive
conductance, fixed root temperature, tip heat load, every history step, final
gradient, final heat flux, and thermal energy. It also checks heat-load
linearity, confirms larger lumped capacity slows the same transient load, and
checks length scaling where lumped capacity rises with length, conductance
falls inversely, and the same finite time window heats more slowly. Every
retained branch also re-derives history maxima and energies from lumped
capacity, checks that the final history frame matches summary fields and final
nodes, and recomputes final element average temperature, gradient, and Fourier
heat flux.

`nonlinear_spring_1d` and `contact_gap_1d` add the first nonlinear mechanical
checks. The hardening spring is compared against the Cardano closed-form root
under load, linear-stiffness, and cubic-stiffness perturbations; the gap stop
checks inactive and active penalty-contact force balance under load, gap, and
contact-stiffness perturbations.

`torsion_1d` adds the rotational shaft path across 1, 2, 4, 8, and 16
elements. It checks closed-form twist, torque, shear stress, and strain energy,
plus torque, length, and polar/section modulus perturbation scaling.

`truss_2d` now has symmetric two-member geometry and load perturbation checks
against closed-form apex displacement, member force, stress, strain, and strain
energy. This specifically exercises coordinate transformation and global
stiffness assembly, not only single-member closed-form behavior.

`truss_3d` extends that check to a symmetric tripod under vertical load. It
checks 3D coordinate transformation, apex displacement, member force, stress,
strain, and strain energy across geometry and load perturbations.

`plane_triangle_2d` and `plane_quad_2d` add retained patch scaling checks for
load, Young's modulus, and thickness perturbations. They verify displacement,
strain, stress, von Mises stress, strain-energy density, and total energy
scale according to the assembled linear-elastic plane-stress model.

`solid_tetra_3d` now adds the constant-strain 3D solid path with a single
free-node tetrahedron. It checks the closed-form displacement, volume,
strain/stress tensor components, von Mises stress, strain-energy density, and
total energy across load, material, Poisson-ratio, height, and base-area
perturbations.

This is deliberately stricter than a sample regression: the uniform axial bar
and the classic Euler-Bernoulli tip-loaded cantilever should remain closed-form
exact under refinement, and the symmetric truss plus single-tetra solid should
remain closed-form under geometry, material, and load perturbation. Any drift
points to bookkeeping, element-length, coordinate-transform, constitutive,
thickness/material scaling, load mapping, force/moment, stress recovery, or
energy accumulation regressions across translational and rotational mechanical
DOFs. The retained axial-bar regression additionally requires summary strain
energy to match both element energy-density integration and
`0.5 * tip_force * tip_displacement`. The beam/frame/torsion classic regression
also keeps an explicit work-energy conjugacy check: beam and frame strain
energy must equal `0.5 * tip_load * tip_displacement`, while torsion strain
energy must equal `0.5 * torque * twist`.

## Source of truth

These baselines are enforced in:

- [accuracy_baselines.rs](../workers/rust/crates/solver/tests/accuracy_baselines.rs)
- [mechanical_convergence.rs](../workers/rust/crates/solver/tests/mechanical_convergence.rs)
- [dynamic_spring_closed_form.rs](../workers/rust/crates/solver/tests/dynamic_spring_closed_form.rs)
- [transient_heat_bar_closed_form.rs](../workers/rust/crates/solver/tests/transient_heat_bar_closed_form.rs)

The first qualification-oriented evidence packet for the 1D closed-form subset
is tracked in:

- [line-field-closed-form-baseline.json](../evidence/operator-qualification/line-field-closed-form-baseline.json)
- [line-field-closed-form-derivation.md](../evidence/operator-qualification/line-field-closed-form-derivation.md)
- [line-field-tolerance-policy.json](../evidence/operator-qualification/line-field-tolerance-policy.json)

Run them with:

```bash
cd workers/rust
cargo test -p kyuubiki-solver --test accuracy_baselines
cargo test -p kyuubiki-solver --test mechanical_convergence
```

For the qualification-oriented subset, retain a release evidence bundle with:

```bash
make capture-line-field-qualification-release-evidence
```

The moxi 2.0.0 retained bundle is checked in at
[line-field-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/line-field-closed-form-release-evidence.json)
and linked from the qualification release record. Its `promotion_summary`
cross-checks the retained review decision, release record, and four promoted
operator ids, so the four 1D closed-form line-field operators now carry
`qualification` evidence only after the retained packet approves them. The
focused heat and electrostatic bar regressions now also cover length scaling:
temperature and potential scale with length under fixed sources, while
gradient, heat flux, electric field, and electric flux density remain pinned
to the same closed-form values. The heat bar regression also checks Fourier
flux recovery and the end-load conservation balance
`heat_flux * area + heat_load = 0`, while re-deriving max temperature, max heat
flux, element length, average temperature, and temperature gradient from public
node and element fields. The electrostatic bar regression also checks element
energy summation, electric-field/flux recovery, the source balance
`electric_flux_density * area + charge = 0`, and the
`0.5 * charge * potential` stored-energy conjugacy. It also re-derives max
potential, element length, average potential, and potential gradient from
public node and element fields.

The second approved qualification packet is
[beam-frame-classic-release-evidence.json](../releases/qualification-evidence/2.0.0/beam-frame-classic-release-evidence.json).
It promotes `solve.beam_1d`, `solve.torsion_1d`, and `solve.frame_2d` after
the retained validation report passes canonical beam, frame, torsion,
sign-convention, and boundary-regression checks. The retained regression also
re-derives displacement, rotation, moment, torque, stress, element-energy
summation, and work-energy consistency from result fields.

The third approved qualification packet is
[electromagnetic-plane-patch-release-evidence.json](../releases/qualification-evidence/2.0.0/electromagnetic-plane-patch-release-evidence.json).
It promotes the electrostatic and magnetostatic triangle/quad plane operators
after retained field, flux, stored-energy, orientation, electrostatic thickness
energy scaling, current-driven magnetostatic inverse-thickness scaling, and
material-provenance checks pass. The retained regression also re-derives
summary maxima, element average potentials, constitutive field/flux laws, and
stored-energy totals directly from the reported node and element fields.

The fourth approved qualification packet is
[thermal-plane-patch-release-evidence.json](../releases/qualification-evidence/2.0.0/thermal-plane-patch-release-evidence.json).
It promotes the heat and thermoelastic triangle/quad plane operators after
retained mesh/refinement, manufactured linear-field refinement, boundary,
thermoelastic stress, heat-flux, heat-plane thickness scaling, thermoelastic
temperature/thickness scaling, and material-provenance checks pass. The retained
regression also re-derives heat-plane summary/flux diagnostics and
thermoelastic displacement, stress, thermal-strain, and strain-energy summaries
from public node and element fields.

The fifth approved qualification packet is
[modal-frame-sanity-release-evidence.json](../releases/qualification-evidence/2.0.0/modal-frame-sanity-release-evidence.json).
It promotes `solve.modal_frame_2d` and `solve.modal_frame_3d` after retained
frequency scaling, mode ordering, restrained DOF, and mode-shape normalization
checks pass, including short-versus-long cantilever period/frequency
monotonicity.

The sixth approved qualification packet is
[stokes-flow-screening-release-evidence.json](../releases/qualification-evidence/2.0.0/stokes-flow-screening-release-evidence.json).
It promotes the Stokes quad and triangle screening operators only for the
retained screening-boundary scope, after boundary, divergence, heterogeneous
material, and 1x1/2x2/4x4 linear-field mesh-refinement checks pass. The retained
quad and triangle screening regressions also cover total viscous-dissipation
invariance under refinement and geometry-area scaling with fixed velocity
boundary data. Every retained branch also re-derives summary maxima, pressure
drop, element average velocity/pressure, Reynolds number, and nonnegative
divergence, shear, stress, and dissipation diagnostics from public result
fields.

The seventh approved qualification packet is
[acoustic-bar-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/acoustic-bar-closed-form-release-evidence.json).
It promotes `solve.acoustic_bar_1d` for the retained 1D linear
frequency-domain duct scope after closed-form pressure, wave-number,
particle-velocity, wave-number frequency-linearity, and damping-loss checks
pass across an octave frequency ladder. The focused closed-form regression now
also checks duct-length perturbation against the same dynamic
closed-form reference, plus pure-source amplitude scaling: pressure and
particle velocity scale linearly, while acoustic intensity and damping loss
scale quadratically. Every retained branch also rechecks summary maxima,
sound-pressure level, speed of sound, wave number, particle velocity, acoustic
intensity, damping loss, element length, and material echo fields from
node/element/material fields.

The eighth approved qualification packet is
[advection-diffusion-bar-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/advection-diffusion-bar-closed-form-release-evidence.json).
It promotes `solve.advection_diffusion_bar_1d` for the retained 1D steady
constant-coefficient transport scope after closed-form flux, Peclet, and
zero-velocity checks pass. The focused closed-form regression also now covers a
fixed-boundary length-scaling case where gradient and diffusive flux scale
inversely with length while Peclet scales linearly, plus a
three-node internal-source fixture where source strength scales the free middle
concentration linearly and cross-sectional area inversely scales the
source-driven concentration increment while the left/right total-flux jump
matches source per area. Every retained branch also checks summary maxima,
element length, average concentration, concentration gradient,
diffusive/advective/total-flux decomposition, and the Peclet formula directly
from public node and element fields.

The ninth approved qualification packet is
[magnetostatic-bar-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/magnetostatic-bar-closed-form-release-evidence.json).
It promotes `solve.magnetostatic_bar_1d` for the retained 1D linear
magnetostatic permeance scope after closed-form potential, field, flux, energy,
and zero-source checks pass. The focused closed-form regression also now
checks source-driven linear/quadratic response, permeability-inverse potential
and energy response while preserving source-controlled flux density, and
area-inverse potential, field, flux-density, and stored-energy response. It
also checks length-linear potential and stored-energy response while preserving
field strength and flux density for the same source, plus the
`0.5 * magnetomotive_source * magnetic_potential` stored-energy conjugacy,
element energy summation, magnetic field/flux recovery, and the source balance
`magnetic_flux_density * area + magnetomotive_source = 0`. It also re-derives
max magnetic potential, element length, average magnetic potential, and magnetic
potential gradient from public node and element fields.

The tenth approved qualification packet is
[spring-1d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/spring-1d-closed-form-release-evidence.json).
It promotes `solve.spring_1d` for the retained 1D linear static series-spring
scope after equivalent-stiffness, member-force, strain-energy, and zero-load
checks pass. The focused closed-form regression also verifies that changing
node spacing changes reported element length without changing the fixed
discrete-stiffness response, while every retained branch checks total energy
against element-energy summation and `0.5 * sum(F*u)`. It also re-derives
element extension, member force, element strain energy, `max_displacement`, and
`max_force` from public input, node, and element fields.

The eleventh approved qualification packet is
[spring-vector-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/spring-vector-closed-form-release-evidence.json).
It promotes `solve.spring_2d` and `solve.spring_3d` for the retained linear
static orthogonal vector-spring scope after displacement, member-force,
extension-sign, fixed-support, and strain-energy checks pass. The focused
closed-form regression also verifies that changing orthogonal anchor distances
changes reported element length without changing the fixed discrete-stiffness
response, while every retained branch checks total energy against element-energy
summation and `0.5 * sum(F*u)`. It also re-derives direction-cosine displacement
projection, member force, element strain energy, `max_displacement`, and
`max_force` from public input, node, and element fields.

The twelfth approved qualification packet is
[thermal-beam-1d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/thermal-beam-1d-closed-form-release-evidence.json).
It promotes `solve.thermal_beam_1d` for the retained linear thermal
free-curvature scope after tip displacement, tip rotation, zero-gradient, and
near-zero internal-force checks pass. The focused closed-form regression also
checks that temperature-gradient and thermal-expansion changes linearly scale
curvature, tip rotation, and tip displacement, while section-depth changes
scale the same free-curvature diagnostics inversely. It also checks length
scaling where curvature remains fixed, tip rotation scales linearly, and tip
displacement scales quadratically. Every retained branch also re-sums member
`strain_energy` into total strain energy to enforce zero-energy free curvature
at summary and element levels. The retained regression also re-derives node
displacement magnitude, max displacement, max rotation, max moment, max stress,
max temperature gradient, and thermal curvature from public result fields.

The thirteenth approved qualification packet is
[contact-gap-1d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/contact-gap-1d-closed-form-release-evidence.json).
It promotes `solve.contact_gap_1d` for the retained 1D penalty stop scope after
inactive-gap, active-contact, penetration, and force-split checks pass. The
focused closed-form regression also checks contact-normal-stiffness scaling,
reduced penetration, the updated spring/contact force split, and geometry-length
invariance for the fixed discrete spring/contact law. Every retained branch also
checks the penalty contact law directly: penetration cutoff, contact force,
active flags/count, `max_contact_force`, and spring/contact/external-load
balance. The retained regression also re-derives spring extension, spring
force, tangent stiffness, max displacement, max spring force, residual bounds,
and converged load-step metadata from public result fields.

The fourteenth approved qualification packet is
[truss-2d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/truss-2d-closed-form-release-evidence.json).
It promotes `solve.truss_2d` for the retained symmetric two-bar truss scope
after axial force, apex displacement, stress, strain, and strain-energy checks
pass. The focused closed-form regression also checks that Young's modulus
inversely scales apex displacement and strain energy while preserving
load-controlled axial force and stress, plus similar-geometry scaling across
member length, apex displacement, and strain energy. It also checks the
work-energy conjugacy `total_strain_energy = 0.5 * apex_load * apex_displacement`.
The retained regression also re-derives max displacement, max stress, max
strain-energy density, total strain energy, Hooke-law stress, axial force,
element energy density, and global external work from public result fields.

The fifteenth approved qualification packet is
[truss-3d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/truss-3d-closed-form-release-evidence.json).
It promotes `solve.truss_3d` for the retained symmetric tripod truss scope
after axial force, apex displacement, stress, strain, and strain-energy checks
pass. The focused closed-form regression also checks that Young's modulus
inversely scales apex displacement and strain energy while preserving
load-controlled leg force and stress, plus similar-geometry scaling across leg
length, apex displacement, and strain energy. It also checks the vertical
load/displacement work-energy conjugacy
`total_strain_energy = 0.5 * apex_load * apex_displacement`. The retained
regression also re-derives max 3D displacement, max stress, max strain-energy
density, total strain energy, Hooke-law stress, axial force, element energy
density, and global external work from public result fields.

The sixteenth approved qualification packet is
[thermal-truss-3d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/thermal-truss-3d-closed-form-release-evidence.json).
It promotes `solve.thermal_truss_3d` for the retained fully restrained
uniform-temperature scope after strain split, stress, axial force, and
strain-energy checks pass. The focused closed-form regression also checks
thermal-expansion and Young's-modulus scaling alongside temperature and area
scaling, plus uniform geometry scaling where member lengths and total energy
scale together. Each retained scaling branch also re-sums member energy from
`strain_energy_density * area * length` and checks max energy density. The
retained regression also re-derives average temperature delta, thermal strain,
mechanical strain, Hooke-law stress, axial force, displacement/temperature
summary maxima, max stress, max axial force, and total strain energy from
public result fields.

The seventeenth approved qualification packet is
[thermal-truss-2d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/thermal-truss-2d-closed-form-release-evidence.json).
It promotes `solve.thermal_truss_2d` for the retained fully restrained
uniform-temperature scope after strain split, stress, axial force, and
strain-energy checks pass. The focused closed-form regression mirrors the 3D
thermal truss lane with temperature, thermal-expansion, Young's-modulus, and
area scaling, plus uniform geometry scaling. Each retained scaling branch also
re-sums member energy from `strain_energy_density * area * length` and checks
max energy density. The retained regression also re-derives average
temperature delta, thermal strain, mechanical strain, Hooke-law stress, axial
force, displacement/temperature summary maxima, max stress, max axial force,
and total strain energy from public result fields.

The eighteenth approved qualification packet is
[thermal-frame-2d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/thermal-frame-2d-closed-form-release-evidence.json).
It promotes `solve.thermal_frame_2d` for the retained fully restrained
uniform-temperature single-member scope after strain split, axial force,
stress, zero-gradient, and strain-energy checks pass. The focused closed-form
regression also checks temperature, thermal-expansion, area, Young's modulus,
and length scaling, with every retained branch re-summing member `strain_energy`
into total strain energy. The retained regression also re-derives displacement,
rotation, average temperature delta, thermal strain, mechanical strain, thermal
curvature, axial stress/force, combined stress, temperature summaries, moment
summary, stress summary, and total strain energy from public result fields.

The nineteenth approved qualification packet is
[thermal-frame-3d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/thermal-frame-3d-closed-form-release-evidence.json).
It promotes `solve.thermal_frame_3d` for the retained fully restrained
single-member thermal-gradient scope after strain split, curvature, moment,
combined-stress, and strain-energy checks pass. The focused closed-form
regression also checks thermal-expansion and Young's-modulus scaling alongside
the retained temperature-gradient, inertia, and length checks, with every
retained branch re-summing member `strain_energy` into total strain energy. The
retained regression also re-derives displacement/rotation magnitudes, average
temperature delta, thermal strain, mechanical strain, both thermal curvatures,
axial stress/force, bending moments/stress, combined stress, temperature
summaries, moment/stress summaries, and total strain energy from public result
fields.

The twentieth approved qualification packet is
[solid-tetra-3d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/solid-tetra-3d-closed-form-release-evidence.json).
It promotes `solve.solid_tetra_3d` for the retained unit constant-strain
tetrahedron scope after reduced stiffness, tip displacement, constitutive
stress, von Mises, and strain-energy checks pass. The focused closed-form
regression now also checks tip-height scaling across volume, displacement,
constant strain/stress recovery, and total strain energy, plus base-area
scaling where wider restrained bases dilute displacement, stress, von Mises
stress, and total energy by the inverse area factor. It also checks the single
free-tip work-energy conjugacy
`total_strain_energy = 0.5 * tip_load * tip_displacement`. The retained
regression now also re-derives node displacement magnitudes, total volume,
von Mises stress, strain-energy density, max summaries, total strain energy,
and external work-energy from public result fields.

The twenty-first approved qualification packet is
[nonlinear-spring-1d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/nonlinear-spring-1d-closed-form-release-evidence.json).
It promotes `solve.nonlinear_spring_1d` for the retained monotone hardening
spring scope after Cardano root, Newton convergence, force balance, tangent
stiffness, and load-step checks pass. The focused closed-form regression also
checks the conservative hardening potential
`U = 0.5 * k * u^2 + 0.25 * c * u^4`, its force/tangent derivatives, and that
changing element length does not alter the fixed discrete hardening-law
response. It also re-derives extension from node displacement, force, tangent
stiffness, max displacement, max force, residual bounds, and converged load-step
metadata from public result fields.

The twenty-second approved qualification packet is
[frame-3d-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/frame-3d-closed-form-release-evidence.json).
It promotes `solve.frame_3d` for the retained single-member 3D cantilever
scope after tip displacement, tip rotation, root moment, bending stress, and
strain-energy checks pass. The focused closed-form regression now also covers
tip-load, bending-inertia, and length scaling for the same retained member, and
re-derives summary displacement, rotation, moment, stress, element-energy
summation, and global work-energy consistency from result fields.

The twenty-third approved qualification packet is
[plane-2d-patch-closed-form-release-evidence.json](../releases/qualification-evidence/2.0.0/plane-2d-patch-closed-form-release-evidence.json).
It promotes `solve.plane_triangle_2d` and `solve.plane_quad_2d` for the
retained small plane-stress patch scope after direct-stiffness displacement,
stress diagnostic, split-triangle, and strain-energy checks pass. The focused
closed-form regression now covers load, thickness, and Young's-modulus scaling
for both triangle and quad retained patch paths, plus similar-geometry scaling
under fixed nodal loads. It also checks the global work-energy conjugacy
`total_strain_energy = 0.5 * sum(load_x * ux + load_y * uy)`. The retained
regression also re-derives node displacement magnitudes, max displacement,
max stress, max strain-energy density, and total strain energy from public
result fields, while preserving the quad split-triangle weighted-diagnostic
contract for nonlinear stress summaries.

## Why these three first

They are the best first return for `v1.x` accuracy work:

- `axial_bar_1d`
  validates the most basic stiffness/force/reaction path
- `thermal_bar_1d`
  validates the simplest trustworthy thermo-mechanical response
- `beam_1d`
  validates flexural response and a more sensitive operator family

Together they cover:

- direct structural displacement/stress
- one-dimensional spring chain extension/force response
- planar spring extension/force distribution
- spatial spring extension/force distribution
- restrained thermal expansion
- free thermal bending from a through-depth temperature gradient
- pure heat conduction with fixed-temperature interpolation
- pure 2D conduction on a triangular patch
- pure 2D conduction on a quad patch
- bending kinematics and moment/stress response

The second wave extends that trust envelope to:

- frame bending with axial-capable line elements
- space-frame bending and restrained thermal space-frame response
- space-truss pyramid response with stable 3D displacement and axial stress paths
- heated triangular truss response in 2D
- triangulated truss load paths
- restrained thermal truss response in 3D
- constant-strain plane triangle response
- bilinear quad patch stress/displacement response
- restrained thermoelastic patch response in triangle elements
- restrained thermoelastic patch response in quad elements
- heated frame response in 2D with thermal curvature and restrained expansion

## Next baselines

After this seed set, the next best families are:

1. newly introduced families beyond the current seed set

## Related docs

- [accuracy-plan.md](accuracy-plan.md)
- [testing-and-ci.md](testing-and-ci.md)
