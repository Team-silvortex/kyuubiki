# Plane 2D Patch Closed-Form Qualification

Candidate: `plane-2d-patch-closed-form`

Operators:

- `solve.plane_triangle_2d`
- `solve.plane_quad_2d`

Current line: `moxi 2.7.x`

## Fixture

The retained validation uses two small plane-stress patch fixtures:

- a unit square decomposed into two constant-strain triangles
- a single native bilinear isoparametric quadrilateral integrated at four
  Gauss points
- distorted `1x1`, `2x2`, and `4x4` Q4 meshes under an affine constant-strain
  field

Both fixtures retain direct-stiffness reference displacements, stress
diagnostics, von Mises stress, and strain-energy totals.

## Invariants

For the triangle path, the retained regression checks the constant-strain
stress state against the planar principal-stress, maximum in-plane shear, von
Mises, and energy-density formulas.

For the quad path, the retained regression checks the Gauss-weighted stress
result and verifies total energy as:

```text
U = energy_density * area * thickness
```

The quad result derives principal and von Mises metrics from the
Jacobian-weighted average stress. Strain-energy density is integrated at all
four Gauss points before it is normalized by element area.

The distorted-mesh patch uses the analytic plane-stress field:

```text
epsilon_x = sigma_x / E
epsilon_y = -nu * sigma_x / E
gamma_xy = 0
```

Every retained refinement reproduces the affine displacement and constant
stress field. Clockwise or folded connectivity is rejected when any
Gauss-point Jacobian is non-positive.

## Scope

This qualifies the current linear small-strain plane-stress patch path for
constant-strain triangles and fully integrated bilinear Q4 elements. It covers
affine distorted-element patch invariance, not arbitrary nonlinear-field mesh
convergence. Plasticity, buckling, reduced integration, incompatible modes,
and large-deformation behavior remain outside this scope.

The retained `2.0.0` release packet remains a historical record of the earlier
split-triangle implementation. Current qualification commands exercise the
native Q4 path, with executed output retained at
`releases/qualification-evidence/2.7.9/plane-2d-patch-isoparametric-evidence.json`.
