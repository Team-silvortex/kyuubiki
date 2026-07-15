# Thermal Plane Boundary Coverage

This note records the review-level boundary coverage currently available for
the 2D heat-plane and thermoelastic-plane patch operators.

## Scope

The evidence applies to compact single-patch or two-triangle fixtures:

- `solve.heat_plane_triangle_2d`
- `solve.heat_plane_quad_2d`
- `solve.thermal_plane_triangle_2d`
- `solve.thermal_plane_quad_2d`

It is boundary and diagnostic coverage, not mesh-convergence evidence.

## Heat Plane Fixtures

The heat-plane review fixtures cover:

- fixed hot and cold temperature boundaries
- one free temperature node resolved by the assembled thermal system
- triangle and quad element geometry
- temperature gradients, heat flux components, heat-flow magnitude, and total
  absolute heat-flow diagnostics

The review fixtures intentionally use scalar linear conductivity and compact
geometry so the expected fields can be checked directly.

## Thermoelastic Plane Fixtures

The thermoelastic-plane review fixtures cover:

- fully restrained displacement boundaries
- uniform temperature delta
- triangle and quad element geometry
- thermal strain, mechanical strain, stress, von Mises, and strain-energy
  diagnostics

The current restrained fixtures are conservative: they catch sign, stress, and
energy regressions, but they do not cover mixed displacement/load boundaries.

## Boundary Gap

Before qualification, the group still needs a retained mixed-boundary fixture
or convergence report that explains how these compact checks generalize beyond
the review cases.
