# Thermal Truss 3D Closed-Form Qualification

This packet qualifies `solve.thermal_truss_3d` only for the retained linear,
fully restrained, uniform-temperature 3D truss fixture.

## Fixture

- All node translations are fixed.
- All members share area `A`, Young's modulus `E`, thermal expansion `alpha`,
  and uniform temperature rise `dT`.
- No external mechanical load is applied.

Because all member end translations are fixed, total axial strain is zero:

```text
epsilon_total = 0
```

The thermal and mechanical strain split is:

```text
epsilon_thermal = alpha dT
epsilon_mechanical = -alpha dT
```

The retained stress, axial force, and energy density checks are:

```text
sigma = -E alpha dT
N = sigma A
u = 0.5 sigma epsilon_mechanical
```

The total strain energy is the sum of `u A L` over every retained member.

This evidence does not claim partial restraint, temperature gradients, buckling,
plasticity, contact, dynamic response, or arbitrary thermal-structural
assemblies.
