# Solid Tetra 3D Closed-Form Qualification

Candidate: `solid-tetra-3d-closed-form`

Operator: `solve.solid_tetra_3d`

Release line: `moxi 2.0.x`

## Fixture

The qualification fixture uses the unit tetrahedron with nodes
`(0,0,0)`, `(1,0,0)`, `(0,1,0)`, and `(0,0,1)`. The first three
nodes are fully restrained. The fourth node is free and receives a
single `z` load `Fz`.

For this geometry the fourth shape-function gradient is `(0, 0, 1)`.
The reduced free `z` stiffness is therefore:

```text
kz = Dzz * V
Dzz = E * (1 - nu) / ((1 + nu) * (1 - 2 * nu))
V = 1 / 6
uz = Fz / kz
```

## Stress And Energy Checks

The only non-zero strain is `epsilon_z = uz`. With isotropic linear
elasticity:

```text
sigma_x = lambda * uz
sigma_y = lambda * uz
sigma_z = Dzz * uz = Fz / V
lambda = E * nu / ((1 + nu) * (1 - 2 * nu))
von_mises = abs(sigma_z - sigma_x)
energy_density = 0.5 * sigma_z * uz
total_energy = energy_density * V
```

The retained solver test checks the restrained base displacement, tip
displacement, constitutive stress components, von Mises stress, energy
density, and total strain energy against these formulas.

## Perturbation And Objectivity Checks

The mechanical convergence lane repeats the analytic comparison across load,
Young's modulus, Poisson ratio, and tetrahedron-height perturbations. It also
rigidly rotates the tetrahedron and applied load about the global `y` axis.
The displacement vector must rotate with the fixture while volume, displacement
magnitude, von Mises stress, strain-energy density, and total strain energy
remain invariant. The rotated force-displacement work is checked independently.

## Scope

This qualifies the current single constant-strain tetrahedron path for
small-strain linear elastic screening. It does not claim full industrial
solid mechanics coverage, multi-element convergence, contact, plasticity,
or large deformation behavior. The perturbation lane establishes parameter
robustness and rigid-rotation objectivity, not mesh-refinement convergence.
