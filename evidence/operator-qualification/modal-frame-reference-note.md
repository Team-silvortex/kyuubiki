# Modal Frame Reference Note

This note records the independent reference used by the
`modal-frame-sanity` qualification candidate.

## Reference Problem

The retained modal frame scope is the linear, undamped generalized eigenproblem:

```text
K phi = omega^2 M phi
```

where `K` is the assembled frame stiffness matrix, `M` is the assembled mass
matrix, `omega` is the natural circular frequency, and `phi` is the mode shape.
The solver reduces fixed degrees of freedom before solving the positive finite
eigenpairs.

## Scaling Reference

For any retained fixture with the same topology and boundary conditions:

```text
K' = alpha K
M' = beta M
omega'^2 = (alpha / beta) omega^2
omega' = sqrt(alpha / beta) omega
f' = sqrt(alpha / beta) f
T' = T / sqrt(alpha / beta)
```

This is independent of the implementation details of the eigensolver. The
current executable checks use it to verify that uniform stiffness and density
changes scale every retained 2D and 3D modal frame frequency, eigenvalue, and
period consistently.

## Cantilever Scope

The retained 2D and 3D modal fixtures use single-member cantilevers. Shorter
members must have higher retained frequencies than longer members under the
same material and section properties. Symmetric 3D bending modes may be
near-degenerate, so the stable ordering claim is non-decreasing for 3D and
strictly increasing for the retained 2D fixture.

## Boundary

This reference supports the current linear modal sanity qualification only. It
does not claim experimental modal correlation, damping, forced response, shell
or solid modal behavior, nonlinear joints, arbitrary mesh convergence, or
stable mode identity across exactly degenerate subspaces.
