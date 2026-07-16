# Spring Vector Closed-Form Qualification Scope

This evidence packet covers `solve.spring_2d` and `solve.spring_3d` for
linear, small-displacement spring networks where the fixture can be reduced to a
single free node connected to orthogonal fixed supports.

## Closed-Form Contract

For the retained 2D case, the free node is connected to fixed supports through
an x-aligned spring and a y-aligned spring. The reduced stiffness matrix is
diagonal:

```text
K = diag(kx, ky)
u = K^-1 f
```

For the retained 3D case, the same construction extends to three orthogonal
springs:

```text
K = diag(kx, ky, kz)
u = K^-1 f
```

The regression checks displacement components, fixed support displacement,
element extension sign, member force, and total strain energy. Total energy is
retained as `0.5 * f dot u`, matching the assembled spring energy.

## Qualification Boundary

This is a closed-form qualification for vector spring projection and assembly,
not a claim that arbitrary nonlinear or contact spring networks are
qualification-grade. The retained scope is intentionally narrow:

- linear spring stiffness
- static equilibrium
- small displacement
- orthogonal support projection
- solver input reliability retained through `spring_input_reliability`

The broader spring-grid and spring-cage review fixtures remain linked so the
qualification packet covers both canonical review behavior and the exact
closed-form vector reduction.
