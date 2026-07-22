# Buckling Frame 2D Linear Eigenvalue Screening

## Scope

`solve.buckling_frame_2d` first solves the supplied 2D frame under its reference
load, derives signed axial member forces from the local static end forces, and
then solves

`K phi = lambda Kg phi`.

Positive compression contributes to `Kg`; tension members remain visible in the
preload report but do not contribute destabilizing stiffness. The result retains
the static solution, every extracted member preload, critical load factors, and
normalized global three-degree-of-freedom mode shapes.

This is a screening capability, not a design-code or qualification solver.

## Retained Checks

- A statically preloaded pinned column converges monotonically to
  `Pcr = pi^2 E I / L^2` over 1, 2, 4, 8, and 16 frame elements.
- Horizontal and vertical representations produce the same critical factor.
- The generalized eigensolver supports semidefinite geometric stiffness caused
  by axial degrees of freedom and emits sorted, normalized multiple modes.
- Extracted compression matches the applied reference load for every column
  element.
- Tension-only, zero-load, and invalid-section reference models are rejected.
- RPC, engine workflow, Web API, Workbench, and official headless SDK routes are
  retained by contract tests.

## Promotion Gaps

- initial geometric imperfections and residual stress
- material plasticity and section yielding interaction
- follower-load and load-path sensitivity
- nonlinear equilibrium continuation and post-buckling response
- independent commercial or laboratory correlation for multi-member frames
- design-code checks, safety factors, and qualification evidence review
