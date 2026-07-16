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

This evidence does not claim partial restraint, temperature gradients, frame
assemblies, geometric nonlinearity, buckling, plasticity, contact, or dynamic
response.
