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

Every returned mode reports `relative_gap_to_next` when the next requested mode
is available and a `direction_assessment`. Gaps at or below `1e-4` are
`clustered`, an assessed separated mode is `isolated`, and a final mode without
a known upper neighbor is `unassessed`. A clustered factor can be reliable even
though its individual vector direction is not unique.

## Retained Checks

- A pinned-pinned uniform column converges monotonically to
  `Pcr = pi^2 E I / L^2` over 1, 2, 4, 8, and 16 elements.
- The critical factor scales linearly with `E` and `I`, inversely with the
  reference compressive force, and with `L^-2`.
- Reduced-space residual norms are finite and returned mode shapes are
  normalized after constrained degrees of freedom are restored.
- A 400-element Euler column forces the shared sparse generalized eigensolver
  and preserves the analytic critical factor.
- Single-mode sparse iteration retains an oversampled block so clustered first
  factors do not degrade into a slowly converging scalar power iteration.
- Two independent 300-element columns with factors separated by only `1e-6`
  force a 1,200-free-DOF single-mode solve; the returned mode selects the softer
  column and retains its Euler factor.
- The independent beam and statically preloaded frame formulations agree on
  the first critical factor and normalized bending-mode shape.
- The `large` release benchmark solves a 2,500-element, 5,000-free-DOF column
  in three consecutive runs. After clustered-mode oversampling, the observed
  median was 51.665 ms with 11 MiB peak RSS, exceeding the former 4,096-DOF
  dense safety limit.
- Invalid topology, non-finite coordinates, non-positive section properties,
  non-positive reference forces, and insufficient restraints are rejected.

## Promotion Gaps

- add imperfection-sensitive nonlinear continuation before making design claims

## Performance Reproduction

```text
cd workers/rust
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile large --repeat 3 \
  --case buckling-beam-1d-large --format table
```
