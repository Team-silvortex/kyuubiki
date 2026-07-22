# Thermal Frame 3D Closed-Form Qualification

This packet qualifies `solve.thermal_frame_3d` only for the retained linear,
fully restrained, single-member 3D frame fixture with uniform temperature rise
and linear temperature gradients.

## Fixture

- Both end nodes are fully fixed in translation and rotation.
- The member is aligned with the global x axis.
- The member has area `A`, Young's modulus `E`, moments of inertia `Iy` and
  `Iz`, section moduli `Sy` and `Sz`, section depths `dy` and `dz`, expansion
  coefficient `alpha`, and length `L`.
- A uniform temperature rise `dT` and gradients `gy`, `gz` are applied.

The full restraint gives:

```text
epsilon_total = 0
epsilon_thermal = alpha dT
epsilon_mechanical = -alpha dT
```

The retained thermal curvatures, force, moments, stress, and energy are:

```text
ky = alpha gy / dy
kz = alpha gz / dz
N = E A alpha dT
My = E Iy kz
Mz = E Iz ky
sigma = N / A + My / Sy + Mz / Sz
U = 0.5 E (A (alpha dT)^2 + Iz ky^2 + Iy kz^2) L
```

## Active Objectivity Hardening

The active qualification profile also runs
`thermal_frame_3d_preserves_coupled_response_under_arbitrary_rotation`. That
cantilever fixture combines an explicit `local_y_axis`, nonuniform nodal
temperatures, both transverse thermal gradients, tip forces, and tip moments.
Two arbitrary 3D rigid rotations transform the geometry, force, moment, and
section orientation together. The resulting translation and rotation vectors
must be covariant while local forces, local moments, stress components, and
strain energy remain invariant.

The same suite includes
`thermal_frame_3d_non_collinear_assembly_preserves_response_under_rotation`, a
three-member spatial chain with four nodes, distinct member directions,
independent section orientations, nonuniform temperatures, dual thermal
gradients, and terminal force and moment. Every nodal displacement and rotation
vector must transform covariantly, and every element-local force, moment,
strain, stress, and energy value must remain invariant under two arbitrary
global rotations.

`thermal_frame_3d_branched_multi_support_assembly_is_objective` adds two fully
fixed spatial supports connected to a shared three-member junction and loaded
stem. It exercises redundant thermal restraint, branch force redistribution,
distinct member orientations, and terminal mechanical loading. The same
full-node covariance and full-element invariance conditions must hold after two
arbitrary rigid rotations. The loaded tip also carries an arbitrary-direction
translational spring whose direction, scalar response, and energy participate
in the same objectivity check.

The optional `local_y_axis` is projected perpendicular to the member axis and
normalized. Non-finite and parallel directions are rejected. Omitting it keeps
the legacy global-reference derivation, which is covered by the protocol
round-trip regression and preserves existing task payloads.

## Directional Elastic Support

Each optional directional spring contributes `k n n^T` to a node's global
translational stiffness after normalizing direction `n`. The solver returns the
normalized direction, directional displacement, restoring reaction, and spring
energy. `thermal_frame_3d_directional_spring_matches_axial_closed_form` checks
an axial member and support against:

```text
u = F / (E A / L + k)
R_spring = -k u
U_total = 0.5 F u = 0.5 (E A / L) u^2 + 0.5 k u^2
```

Out-of-range nodes, non-positive stiffness, non-finite direction, and zero
direction are rejected before assembly. Omitting the spring collection retains
legacy payload behavior.

## Multi-Element Convergence

`thermal_frame_3d_quadratic_fields_converge_at_second_order` uses an explicitly
oriented fixed-free frame with quadratic axial temperature and independent
quadratic local-y and local-z thermal gradients. The manufactured tip axial
displacement, two transverse displacements, and two bending rotations are
analytic integrals of thermal strain and curvature. Refinement through 1, 2, 4,
8, and 16 elements must reduce every response error at the expected
second-order rate.

This evidence does not claim exact rigid directional constraints, rotational
springs, torsion-dominant response, geometric nonlinearity, buckling,
plasticity, contact, or dynamic response.
