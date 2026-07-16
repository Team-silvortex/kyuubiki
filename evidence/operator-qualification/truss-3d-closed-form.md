# Truss 3D Closed-Form Qualification

This packet qualifies `solve.truss_3d` only for the retained linear, pin-jointed,
small-displacement symmetric tripod fixture.

## Fixture

- Three base nodes are fully fixed on an equilateral triangle of radius `r`.
- One free apex node is at height `h` above the base centroid.
- Three identical truss members connect each base node to the apex.
- A vertical load `P` is applied to the apex.

The member length is:

```text
L = sqrt(r^2 + h^2)
```

The vertical direction cosine is:

```text
n = h / L
```

## Expected Response

Symmetry requires zero apex `ux` and `uy`. Vertical equilibrium gives the same
axial force in all three legs:

```text
N = P / (3 n)
```

With area `A` and Young's modulus `E`, the expected stress, strain, and apex
vertical displacement are:

```text
sigma = N / A
epsilon = sigma / E
uz = P L / (3 E A n^2)
```

The retained energy balance is:

```text
U = 3 * 0.5 * sigma * epsilon * A * L
```

This evidence does not claim arbitrary 3D topology, member buckling, geometric
nonlinearity, dynamic response, damaged members, or joint eccentricity.
