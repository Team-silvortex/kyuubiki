# Truss 2D Closed-Form Qualification Scope

This evidence packet covers `solve.truss_2d` for a retained symmetric two-bar
pin-jointed truss with two fixed base nodes and one vertically loaded apex.

## Closed-Form Contract

For bar length `L`, height `h`, area `A`, modulus `E`, and vertical load `P`,
the retained symmetric fixture has:

```text
sin(theta) = h / L
N = P / (2 * sin(theta))
uy = P * L / (2 * E * A * sin(theta)^2)
stress = N / A
strain = stress / E
```

The regression checks support displacements, apex horizontal symmetry,
closed-form apex vertical displacement, equal member axial force, stress,
strain, strain-energy density, and total strain energy.

## Qualification Boundary

This qualification is intentionally narrow:

- linear elastic material
- small displacement
- pin-jointed 2D truss elements
- symmetric two-bar fixture
- static vertical apex load

It does not qualify arbitrary truss topology, geometric nonlinearity, buckling,
dynamic response, damaged members, or 3D space truss behavior.
