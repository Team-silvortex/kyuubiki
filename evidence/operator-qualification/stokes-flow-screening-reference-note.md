# Stokes Flow Screening Reference Note

This note records the independent reference for the
`screening-cfd-boundary` qualification candidate.

## Reference Problem

The retained screening scope is steady, incompressible, low-Reynolds-number
Stokes flow:

```text
div(u) = 0
-grad(p) + mu laplacian(u) + b = 0
```

The current compact fixtures do not solve a production Navier-Stokes problem.
They retain diagnostic plumbing for velocity, pressure, divergence, shear,
viscous stress, Reynolds number, and dissipation under a linear Stokes field.

## Manufactured Linear Field

The retained refinement reference uses the unit-square field:

```text
u = y
v = 0
p = 0
```

This field has:

```text
du/dx = 0
du/dy = 1
dv/dx = 0
dv/dy = 0
div(u) = du/dx + dv/dy = 0
shear_rate = 1
```

For the same geometry and boundary values, both quad and triangle screening
paths must retain zero divergence and the same shear-rate diagnostics across
1x1, 2x2, 4x4, and 8x8 refinements.

## Material Scaling Reference

For the same linear velocity field, viscosity scales viscous shear stress and
viscous dissipation linearly. Density scales the reported Reynolds number but
does not change the prescribed velocity, shear rate, or viscous stress.

## Boundary

This reference qualifies only the retained Stokes screening boundary. It does
not claim transient flow, turbulence, compressible flow, nonlinear advection,
production CFD tolerances, or arbitrary mesh-convergence accuracy.
