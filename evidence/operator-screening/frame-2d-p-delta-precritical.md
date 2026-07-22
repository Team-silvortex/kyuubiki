# Frame 2D P-Delta Precritical Screening

## Scope

`solve.frame_2d_p_delta` combines the statically preloaded frame buckling model
with either a selected eigenmode or a user-supplied normalized nodal
imperfection field. The field is scaled to the requested maximum translational
amplitude. Its default `linearized_p_delta` mode solves the second-order
equilibrium at each load factor `lambda`

`(K - lambda Kg) u = lambda F + lambda Kg u0`.

The optional `corotational` mode treats the imperfect mesh as stress-free,
updates each member's chord and relative end rotations, and follows the same
precritical load path with incremental Newton iterations and backtracking. Its
energy-consistent analytic tangent is cross-checked component by component
against central differences.

Both modes are elastic precritical screening paths. Neither is a
material-plasticity model, arc-length continuation, or post-buckling solver.

## Retained Checks

- A pinned Euler column uses its first eigenmode as `u0` and matches the secant
  amplification `1 / (1 - lambda / lambda_cr)` over eight load steps.
- A three-member portal accepts an explicit nodal sway imperfection, preserves
  its requested amplitude, and retains critical factors, amplification history,
  and displacement norms under arbitrary rigid rotation.
- The corotational portal converges every incremental equilibrium step, requires
  genuine Newton iterations, and preserves its response under arbitrary rigid
  rotation. A rigidly rotated element produces no spurious internal force, and
  the low-load response recovers the linearized P-Delta result.
- The analytic corotational tangent matches a retained central-difference
  Jacobian at a finite translated and rotated element state.
- Explicit fields with the wrong DOF count, non-finite entries, or no
  translational imperfection are rejected rather than falling back to a mode.
- Amplification increases monotonically and reduced equilibrium residuals stay
  below the retained numerical tolerance.
- The default path ends at `0.8 lambda_cr` when no maximum factor is supplied.
- User paths at or above `0.95 lambda_cr` are rejected before tangent solution;
  the `0.95` limit is returned as read-only result metadata.
- The workflow executor and agent RPC retain imperfection amplitude, mode index,
  explicit shape, source kind, load-step count, full displacement history, and
  solver provenance.

## Promotion Gaps

- corotational large-model performance qualification
- adaptive load stepping and arc-length equilibrium continuation
- material yielding, residual stress, and section interaction
- post-critical branch switching and external reference correlation

## Performance Reproduction

On the local macOS release build, the retained medium case (121 nodes, 120
elements, 363 total DOFs, eight load steps) completed with a `161.8531 ms`
median over five repetitions and `9 MiB` peak RSS. The precritical tangent
path uses banded Cholesky when its storage budget permits and falls back to the
general sparse SPD path for broader topologies.

The retained large case (2,501 nodes, 2,500 elements, 7,503 total DOFs) reached
a `775.8730 ms` median over three repetitions with `19 MiB` peak RSS.

```text
cd workers/rust
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile medium --repeat 5 \
  --case frame-2d-p-delta-medium --format table
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile large --repeat 3 \
  --case frame-2d-p-delta-large --format table
```
