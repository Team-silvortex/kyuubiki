# Electromagnetic Plane Field Energy Evidence

This note records the current review-level energy checks for the 2D
electrostatic and magnetostatic plane patch operators.

## Scope

The evidence applies only to linear, scalar material parameters on compact
single-patch triangle and quad fixtures:

- `solve.electrostatic_plane_triangle_2d`
- `solve.electrostatic_plane_quad_2d`
- `solve.magnetostatic_plane_triangle_2d`
- `solve.magnetostatic_plane_quad_2d`

It is review evidence, not mesh-convergence evidence and not a production
qualification claim.

## Electrostatic Energy

The electrostatic fixtures use a fixed potential gradient. The solver reports:

- electric field `E = -grad(phi)`
- electric flux density `D = epsilon * E`
- energy density `u_e = 0.5 * epsilon * |E|^2`
- stored energy `U_e = u_e * area * thickness`

The current review fixtures use `epsilon = 3.0`, `|E| = 8.0`, and
`thickness = 0.1`.

Expected energy density:

```text
u_e = 0.5 * 3.0 * 8.0^2 = 96.0
```

Expected stored energy:

```text
triangle: U_e = 96.0 * 0.5 * 0.1 = 4.8
quad:     U_e = 96.0 * 1.0 * 0.1 = 9.6
```

## Magnetostatic Energy

The magnetostatic fixtures use scalar permeability `mu = 4*pi*1e-7` and a
source patch with `H = 100.0`.

The solver reports:

- vector potential gradient
- magnetic flux density `B = mu * H`
- magnetic energy density `u_m = 0.5 * B * H`
- stored energy `U_m = u_m * area * thickness`

Expected flux density:

```text
B = mu * 100.0 = 0.0001256637061435917
```

Expected energy density:

```text
u_m = 0.5 * B * 100.0 = 0.006283185307179585
```

Expected stored energy:

```text
triangle: U_m = u_m * 0.5 * 0.1 = 0.00031415926535897925
quad:     U_m = u_m * 1.0 * 0.1 = 0.0006283185307179585
```

## Tolerance Boundary

### Nonuniform Split-Quad Regression (2026-09-20)

The formulas above apply directly to a constant-gradient patch. For a quad
split into two constant-gradient triangles, the reported field vector remains
area-averaged but the energy must be integrated before averaging:

```text
c = epsilon (electrostatic) or 1 / mu (magnetostatic)
U = thickness * c / 2 * (area_1 * |grad_1|^2 + area_2 * |grad_2|^2)
u = U / (thickness * (area_1 + area_2))
```

For the unit square with alternating scalar values 0, 1, 0, 1 around its
boundary, gradients are (1, -1) and (-1, 1). Each area is 0.5. With c = 2.5
and thickness = 0.25, the integrated energy is 0.625, even though the averaged
field vector is zero. The old squared-average implementation incorrectly
returned zero. For areas 1 and 0.5 with gradients (0.5, -0.5) and (-2, 2),
the corresponding analytic energy is 1.40625, not the equal-area average.

Both examples are checked against explicit triangle meshes and reversed
connectivity in `workers/rust/crates/solver/tests/scalar_plane_reference_invariance.rs`.
This is additional regression evidence, not retrospective extension of the
original review packet's qualification scope.

### Retained Review Tolerances

The review fixtures use tight floating-point tolerances because the current
patches are compact and closed-form. Reusing these tolerances for rotated
patches, multi-element meshes, nonlinear materials, time-varying fields, or
coupled high-frequency electromagnetics requires new evidence.
