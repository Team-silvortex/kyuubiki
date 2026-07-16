# Plane 2D Patch Closed-Form Qualification

Candidate: `plane-2d-patch-closed-form`

Operators:

- `solve.plane_triangle_2d`
- `solve.plane_quad_2d`

Release line: `moxi 2.0.x`

## Fixture

The retained validation uses two small plane-stress patch fixtures:

- a unit square decomposed into two constant-strain triangles
- a single quadrilateral interpreted through the solver's split-triangle
  contract

Both fixtures retain direct-stiffness reference displacements, stress
diagnostics, von Mises stress, and strain-energy totals.

## Invariants

For the triangle path, the retained regression checks the constant-strain
stress state against the planar principal-stress, maximum in-plane shear, von
Mises, and energy-density formulas.

For the quad path, the retained regression checks the split-triangle weighted
stress result and verifies total energy as:

```text
U = energy_density * area * thickness
```

The quad result intentionally does not recompute von Mises from the weighted
stress state; the solver reports the area-weighted split-triangle diagnostic.

## Scope

This qualifies the current linear small-strain plane-stress patch path for
constant-strain triangles and split-triangle quads. It does not claim mesh
convergence, high-order quadrature, distorted-element robustness, plasticity,
buckling, or large-deformation behavior.
