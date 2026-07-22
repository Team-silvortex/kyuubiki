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
- A two-column portal retains its extracted column compression and critical
  factor when geometry and loads are rigidly rotated in the global plane.
- The unloaded portal beam remains visible in the preload report without being
  activated as destabilizing compression.
- The generalized eigensolver supports semidefinite geometric stiffness caused
  by axial degrees of freedom and emits sorted, normalized multiple modes.
- Models above 512 free degrees of freedom use sparse `K^-1 Kg` subspace
  iteration. Narrow-band stiffness matrices reuse a memory-bounded symmetric
  band Cholesky factor instead of materializing a dense generalized operator.
- A 400-element Euler column forces the sparse path and retains the analytic
  critical factor within the closed-form screening tolerance.
- The statically preloaded frame and independent prescribed-force beam
  formulations agree on the first critical factor and bending-mode shape.
- The `large` release benchmark solves a 2,500-element, 7,500-free-DOF column
  in three consecutive runs. The observed median was 488.686 ms with 16 MiB
  peak RSS, exceeding the former 4,096-DOF dense safety limit.
- Large ill-conditioned systems may converge at an explicit `2e-3` relative
  residual roundoff floor only when the critical factor also changes by no
  more than `1e-6`; the measured residual remains in the result.
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

## Performance Reproduction

```text
cd workers/rust
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile large --repeat 3 \
  --case buckling-frame-2d-large --format table
```
