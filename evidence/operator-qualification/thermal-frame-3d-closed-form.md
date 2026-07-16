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

This evidence does not claim partial restraint, arbitrary 3D frame assemblies,
torsion-dominant response, geometric nonlinearity, buckling, plasticity,
contact, or dynamic response.
