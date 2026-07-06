# Line-Field Closed-Form Qualification Notes

This note documents the analytic references behind the
`line-field-closed-form` qualification evidence artifact.

Artifact:

- `evidence/operator-qualification/line-field-closed-form-baseline.json`
- `evidence/operator-qualification/line-field-tolerance-policy.json`

Automation:

- `make check-line-field-closed-form-baseline`
- `make capture-line-field-qualification-provenance`
- `make capture-line-field-qualification-release-evidence`
- `cargo test -p kyuubiki-solver --test accuracy_baselines line_1d`

Scope:

- Linear, small-response, one-dimensional finite-element fields.
- Uniform material constants inside each element.
- No nonlinear constitutive behavior, contact, large deformation, radiation,
  or transient response.
- These cases are qualification inputs, not production validation by
  themselves.

## `solve.bar_1d`

Case: `axial_bar_1d_closed_form`, one uniform axial bar, fixed at the left end,
loaded by a tensile force at the right end.

Definitions:

- `L = 1.0`
- `A = 0.01`
- `E = 210000000000.0`
- `F = 1000.0`

Closed-form references:

- `tip_displacement = F * L / (A * E)`
- `max_stress = F / A`
- `reaction_force = -F`

Expected values:

- `tip_displacement = 4.761904761904762e-7`
- `max_stress = 100000.0`
- `reaction_force = -1000.0`

The displacement tolerance is absolute `1e-12`; stress and reaction tolerances
are absolute `1e-6`. These are regression tolerances for a one-element
closed-form case, not general engineering tolerances.

## `solve.thermal_bar_1d`

Case: `thermal_bar_1d_restrained_uniform_rise`, one uniform thermoelastic bar
with both ends fixed and a uniform temperature rise.

Definitions:

- `L = 1.0`
- `A = 0.01`
- `E = 210000000000.0`
- `alpha = 0.000012`
- `delta_temperature = 40.0`

Closed-form references:

- `free_expansion = alpha * delta_temperature * L`
- `restrained_stress_magnitude = E * alpha * delta_temperature`
- `restrained_axial_force_magnitude = restrained_stress_magnitude * A`

Expected values:

- `max_displacement = 0.0`
- `max_stress = 100800000.0`
- `max_axial_force = 1008000.0`
- `max_temperature_delta = 40.0`
- element stress sign is compressive

The zero-displacement and temperature tolerances are absolute `1e-12`; stress
and axial-force magnitudes use relative `1e-9`. The relative tolerance is used
because the magnitudes are large and should remain stable across formatting or
minor floating-point implementation changes.

## `solve.heat_bar_1d`

Case: `heat_bar_1d_two_element_gradient`, two equal heat-conduction elements
with fixed end temperatures and no internal heat load.

Definitions:

- node positions are `0.0`, `1.0`, `2.0`
- fixed end temperatures are `100.0` and `20.0`
- conductivity is `45.0`
- area is `0.01`

Closed-form references:

- `middle_temperature = (T_left + T_right) / 2`
- `temperature_gradient = (T_right - T_left) / L_total`
- `heat_flux = -conductivity * temperature_gradient`

Expected values:

- `max_temperature = 100.0`
- `max_heat_flux = 1800.0`
- `node_1_temperature = 60.0`
- `element_0_temperature_gradient = -40.0`
- `element_1_temperature_gradient = -40.0`

The temperature and gradient tolerances are absolute `1e-12`; heat flux uses
absolute `1e-9`. This pins the exact two-element interpolation path and flux
sign convention used by the current solver.

## `solve.electrostatic_bar_1d`

Case: `electrostatic_bar_1d_two_element_gradient`, two equal electrostatic
elements with fixed end potentials and no free charge.

Definitions:

- node positions are `0.0`, `1.0`, `2.0`
- fixed end potentials are `12.0` and `4.0`
- permittivity is `3.0`
- area is `0.01`

Closed-form references:

- `middle_potential = (V_left + V_right) / 2`
- `electric_field = -(V_right - V_left) / L_total`
- `flux_density = permittivity * electric_field`

Expected values:

- `max_potential = 12.0`
- `max_electric_field = 4.0`
- `max_flux_density = 12.0`
- `node_1_potential = 8.0`
- `element_0_electric_field = 4.0`
- `element_1_electric_field = 4.0`

All numeric tolerances are absolute `1e-12`. This case validates the simple
potential interpolation path, electric-field sign convention, and flux-density
scaling in the 1D electrostatic operator.

## Remaining Qualification Blockers

Before any of these operators should be promoted from `review` to
`qualification`, the evidence packet still needs:

- release-gated regression output retained as a release artifact
