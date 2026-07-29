# Thermal Plane Q4 Closed-Form Reference

Candidate: `thermal-plane-patch`

Operator: `solve.thermal_plane_quad_2d`

Current line: `moxi 2.7.x`

## Element Contract

The current thermoelastic quadrilateral is a four-node bilinear isoparametric
plane-stress element. Its shape functions on `[-1, 1] x [-1, 1]` are:

```text
N1 = (1 - xi)(1 - eta) / 4
N2 = (1 + xi)(1 - eta) / 4
N3 = (1 + xi)(1 + eta) / 4
N4 = (1 - xi)(1 + eta) / 4
```

Geometry and temperature use the same interpolation:

```text
x(xi, eta)       = sum(Ni * xi_node)
y(xi, eta)       = sum(Ni * yi_node)
delta_T(xi, eta) = sum(Ni * delta_Ti)
```

At the four `(+/-1/sqrt(3), +/-1/sqrt(3))` Gauss points, the implementation
forms the physical strain-displacement matrix `B` from the isoparametric
Jacobian and integrates:

```text
K    = thickness * sum(B^T * D * B * det(J))
f_th = thickness * sum(B^T * D * epsilon_th * det(J))

epsilon_th = alpha * delta_T * [1, 1, 0]^T
```

Every Gauss-point Jacobian must be finite and positive.

## Free Uniform Expansion

For constant `delta_T` and the minimum supports needed to remove rigid-body
motion, the analytic displacement field is:

```text
u = alpha * delta_T * x
v = alpha * delta_T * y
```

Because the Q4 geometry and displacement fields share the same shape
functions, this affine field is represented exactly even on a distorted
isoparametric mesh. Therefore:

```text
epsilon_total      = alpha * delta_T * [1, 1, 0]^T
epsilon_mechanical = 0
stress             = 0
strain_energy      = 0
```

The retained regression checks this invariant on distorted `1x1`, `2x2`, and
`4x4` meshes.

## Restrained Linear Temperature

For a fully restrained element with:

```text
delta_T(x) = T0 + gradient * x
```

the total strain is zero. Under plane stress, the area-averaged equal biaxial
stress is:

```text
average_delta_T = T0 + gradient * centroid_x
sigma_x = sigma_y = -E * alpha * average_delta_T / (1 - nu)
tau_xy = 0
```

The retained trapezoid has vertices `(0,0)`, `(2,0)`, `(2.3,1)`, `(0,1)`,
area `2.15`, centroid `x = 1.0767441860465117`, and uses `T0 = 20`,
`gradient = 8`. Its exact physical-area average temperature delta is
`28.613953488372093`.

This reference checks temperature interpolation and Jacobian weighting in
stress recovery independently of the free-expansion load-balance fixture.

## Scope

These references establish linear small-strain plane-stress behavior for a
fully integrated bilinear thermoelastic Q4. They do not qualify plasticity,
large deformation, reduced integration, arbitrary nonlinear temperature
fields, or mixed thermomechanical contact.
