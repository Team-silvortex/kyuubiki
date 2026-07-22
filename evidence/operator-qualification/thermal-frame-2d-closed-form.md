# Thermal Frame 2D Closed-Form Qualification

This packet qualifies `solve.thermal_frame_2d` only for the retained linear,
fully restrained, uniform-temperature single-member 2D frame fixture.

## Fixture

- Both end nodes are fully fixed in translation and rotation.
- The member has area `A`, Young's modulus `E`, expansion coefficient `alpha`,
  and length `L`.
- A uniform temperature rise `dT` is applied.
- The transverse temperature gradient is zero.

The end constraints force zero total axial strain:

```text
epsilon_total = 0
epsilon_thermal = alpha dT
epsilon_mechanical = -alpha dT
```

The retained axial force, stress, and strain-energy checks are:

```text
N = E A alpha dT
sigma = E alpha dT
U = 0.5 E A (alpha dT)^2 L
```

With zero thermal gradient, the expected curvature, end moments, shears, and
bending stress are zero.

## Active Objectivity Hardening

The active qualification profile also runs
`thermal_frame_2d_preserves_coupled_response_under_rigid_rotation`. That
cantilever fixture combines nonuniform nodal temperatures, a transverse thermal
gradient, a tip force, and a tip moment. Multiple in-plane rigid rotations must
preserve translated displacement covariance, rotation response, local axial and
shear forces, local moments, stress components, and strain energy.

This extra regression hardens the operator beyond the retained closed-form
fixture without widening the historical release decision. This evidence still
does not widen the historical release decision.

## Multi-Element Convergence

`thermal_frame_2d_quadratic_field_converges_at_second_order` uses a fixed-free
multi-element frame with quadratic axial temperature and quadratic transverse
thermal-gradient fields. Nodal temperatures supply the element endpoint
average and element gradients sample each midpoint. For length `L`, the
manufactured tip response is integrated analytically:

```text
ux(L) = alpha integral_0^L T(x) dx
rz(L) = alpha / depth integral_0^L g(x) dx
uy(L) = alpha / depth integral_0^L (L - x) g(x) dx
```

Refinement through 1, 2, 4, 8, and 16 elements must reduce all three errors at
the expected second-order rate. This evidence still does not claim arbitrary
temperature fields, geometric nonlinearity, buckling, plasticity, contact, or
dynamic response.
