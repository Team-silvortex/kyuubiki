# Magnetostatic Bar Closed-Form Evidence

This evidence applies to `solve.magnetostatic_bar_1d` for the retained 1D
linear single-core permeance scope.

The retained fixture uses a two-node, one-element magnetic bar with one
grounded magnetic-potential node and one magnetomotive source node. The free
magnetic potential follows the scalar permeance balance:

```text
permeance = permeability * area / length
magnetic_potential_source = magnetomotive_source / permeance
magnetic_potential_gradient = magnetic_potential_source / length
magnetic_field_strength = -magnetic_potential_gradient
magnetic_flux_density = permeability * magnetic_field_strength
stored_energy = 0.5 * permeability * magnetic_field_strength^2 * area * length
```

The retained regression
`workers/rust/crates/solver/tests/magnetostatic_bar_closed_form.rs` checks
that the solver matches the scalar closed form across different length, area,
permeability, and source magnitudes. It also verifies that a zero-source
fixture reports zero potential, field strength, flux density, and stored
energy.

The retained refinement regression
`workers/rust/crates/solver/tests/magnetostatic_bar_refinement.rs` fixes the
magnetic potential at every node for 1, 2, 4, 8, and 16 element chains. It
preserves nodal potential, potential gradient, magnetic field strength, flux
density, and stored energy for the manufactured linear field.

Scope limits:

- This qualifies the current 1D linear magnetostatic bar scope.
- It does not claim nonlinear magnetic materials, hysteresis, saturation,
  eddy currents, time-varying fields, or multidimensional magnetic circuits.
