# Buckling Beam 1D Linear Eigenvalue Screening

## Scope

`solve.buckling_beam_1d` solves the generalized symmetric eigenproblem

`K phi = lambda Kg phi`

for an Euler-Bernoulli beam-column assembled from two-degree-of-freedom line
elements. Each element carries a positive reference compressive force. The
reported eigenvalue is the multiplier on that reference force pattern.

This is a screening capability. It does not include geometric imperfections,
material plasticity, follower loads, contact, large rotations, or post-buckling
equilibrium paths.

## Retained Checks

- A pinned-pinned uniform column converges monotonically to
  `Pcr = pi^2 E I / L^2` over 1, 2, 4, 8, and 16 elements.
- The critical factor scales linearly with `E` and `I`, inversely with the
  reference compressive force, and with `L^-2`.
- Reduced-space residual norms are finite and returned mode shapes are
  normalized after constrained degrees of freedom are restored.
- Invalid topology, non-finite coordinates, non-positive section properties,
  non-positive reference forces, and insufficient restraints are rejected.

## Promotion Gaps

- cross-check against an independent frame stability implementation
- derive element reference forces from a preceding static frame solve
- add 2D frame-column orientation and multi-member global modes
- add imperfection-sensitive nonlinear continuation before making design claims
