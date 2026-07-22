# Thermal Truss 2D Closed-Form Qualification

This packet qualifies `solve.thermal_truss_2d` only for the retained linear,
fully restrained, uniform-temperature 2D truss fixture.

## Fixture

- All node translations are fixed.
- All members share area `A`, Young's modulus `E`, thermal expansion `alpha`,
  and uniform temperature rise `dT`.
- No external mechanical load is applied.

Because every member endpoint is fixed, the total axial strain is zero:

```text
epsilon_total = 0
```

The thermal and mechanical split is:

```text
epsilon_thermal = alpha dT
epsilon_mechanical = -alpha dT
```

The retained stress, axial force, and strain-energy-density checks are:

```text
sigma = -E alpha dT
N = sigma A
u = 0.5 sigma epsilon_mechanical
```

The retained total strain energy is the sum of `u A L` over each member.

## Coupled Objectivity Check

The active qualification profile also solves a two-leg triangle with fixed base
nodes, a free apex, non-uniform nodal temperatures, and an apex mechanical load.
The geometry and load vector are rigidly rotated in the plane. Apex displacement
must rotate with the model while member lengths, temperatures, thermal and
mechanical strains, stress, axial force, and energy remain invariant.

This evidence does not claim partial restraint, mixed thermal loading,
temperature gradients, buckling, plasticity, contact, or dynamic response.
