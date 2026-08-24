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

## Multi-Element Patch And Equilibrium Checks

The current-line depth lane decomposes a rectangular solid into six linear
tetrahedra per structured cell and repeats the uniform uniaxial-traction patch
at `1`, `2`, `4`, and `8` cells per axis. Consistent nodal loads are derived
from the two boundary triangles on every loaded face cell. The independent
reference field is:

```text
epsilon_x = sigma / E
epsilon_y = epsilon_z = -nu * epsilon_x
u = (epsilon_x * x, epsilon_y * y, epsilon_z * z)
sigma_x = sigma
sigma_y = sigma_z = tau_xy = tau_yz = tau_zx = 0
U = 0.5 * sigma * epsilon_x * volume
```

Every refinement must reproduce the affine displacement field, uniaxial
stress, total volume, and strain energy. The public result also exposes
constraint reactions, maximum free-DOF residual, and resultant force balance.
The test requires applied load plus support reaction to close and verifies that
old serialized results without the additive diagnostics still deserialize with
safe defaults.

## Non-Affine Pure-Bending Convergence

The next depth lane uses an exact three-dimensional pure-bending elasticity
field on a rectangular solid centered on all three axes. With curvature
`kappa`, the manufactured displacement and stress are:

```text
ux = -kappa * x * z
uy = nu * kappa * y * z
uz = 0.5 * kappa * (x^2 - nu * y^2 + nu * z^2)
sigma_x = -E * kappa * z
sigma_y = sigma_z = tau_xy = tau_yz = tau_zx = 0
```

The two end faces receive the exact linear traction `tx = nx * sigma_x` using
the consistent triangle load integral. The loads have zero resultant force and
equal opposite bending moments. Six zero-valued scalar anchors remove only the
rigid-body nullspace and are selected where the analytic displacement is zero.

At `2`, `4`, `8`, and `16` cells per axis, displacement L2 errors contract from
`0.6156` to `0.3191`, `0.1111`, and `0.0308`; stress L2 errors contract from
`0.6723` to `0.4640`, `0.2738`, and `0.1460`; strain-energy errors contract from
`0.6093` to `0.2961`, `0.1010`, and `0.0282`. The finest mesh contains `4,913`
nodes and `24,576` tetrahedra. The gate also requires negligible anchor
reaction, free-DOF residual, and resultant force imbalance. This is a genuine
non-affine convergence check: unlike the constant-stress patch, the linear
tetrahedral basis cannot represent the quadratic displacement exactly.

## Warped Mesh And Quality Visibility

The same pure-bending field is repeated on `4`, `8`, and `16` meshes after a
deterministic interior-node warp of up to `22%` of local spacing. The exterior
geometry, exact end traction, and nullspace anchors remain unchanged. Minimum
mean-ratio quality remains `0.2962`, `0.2850`, and `0.2827`; displacement errors
contract from `0.3864` to `0.1521` and `0.0456`, stress errors from `0.5169` to
`0.3355` and `0.1900`, and energy errors from `0.3388` to `0.1315` and `0.0399`.

Each element now reports normalized tetrahedral mean-ratio quality. The result
summarizes minimum quality, counts below the visible `0.20` distortion and
`0.05` severe-distortion thresholds, and emits stable watch terms. Poisson
ratio `nu >= 0.45` similarly emits
`near_incompressible_volumetric_locking_risk`; this is a warning, not a false
claim that the constant-strain formulation avoids locking. Shape degeneracy is
scale-independent: a geometrically regular tetrahedron scaled to `1e-9` still
solves, a solvable severe sliver is reported, and mean-ratio quality at or below
`1e-12` fails closed before factorization.

## Topology And Restraint Preflight

The model is partitioned into connected components from tetrahedral incidence
before assembly. Orphan nodes fail immediately. For each component, constrained
scalar degrees of freedom are projected onto the three translations and three
infinitesimal rotations about a centered, scale-normalized frame. The resulting
constraint matrix must have rigid-body rank `6/6`; simply counting six fixed
degrees of freedom is not sufficient. A regression demonstrates that two fully
fixed points still leave rotation about their connecting line and are therefore
rejected at rank `5/6`.

A second regression rejects an independently floating component before
factorization, while two independently restrained components solve together as
one block system and report `connected_component_count = 2`. Reordering node
storage and remapping element connectivity preserves nodal displacement by id,
maximum von Mises stress, and total strain energy. These checks remove hidden
dependence on global indexing and make disconnected-domain intent explicit.

## Input Reliability

The retained input reliability regression
`workers/rust/crates/solver/tests/solid_tetra_input_reliability.rs` rejects
non-finite node coordinates and loads, missing or duplicate topology,
zero-volume or scale-relative near-degenerate tetrahedra, invalid Young's
modulus, and invalid Poisson ratio values before qualification evidence is
accepted.

## Scope

This qualifies the constant-strain tetrahedron for small-strain linear elastic
single-element references, affine multi-element patch assembly, and the scoped
manufactured pure-bending convergence ladder on regular and deterministically
warped interior meshes. It also qualifies arbitrary node/index ordering and
multiple independently restrained connected components. It does not claim a
general unstructured mesh generator or broad connectivity-family corpus,
nearly incompressible accuracy, plasticity, contact, body-force integration,
native surface-traction assembly, or large deformation. Broader independent
3D references, stabilized incompressible formulations, and higher-order solids
remain separate work.
