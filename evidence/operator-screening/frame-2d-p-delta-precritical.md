# Frame 2D P-Delta Stability Screening

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
against central differences. Failed nominal steps are bisected and retried from
the last accepted state, up to the configured `max_step_cutbacks` limit. The
defaults are a `1e-7` relative force tolerance and 12 cutbacks; explicit user
values are applied without hidden model-size scaling.

Corotational kinematics also accepts experimental `arc_length` path control.
Its spherical predictor-corrector solves displacement and load-factor
increments together, preserves the previous generalized path direction, and
reports the normalized arc-length constraint error on every step. The default
radius is derived from the requested reference path span, step count, initial
load direction, and explicit or automatically derived load scale. User-supplied
`arc_length_radius` and `arc_length_load_scale` values remain visible in the
request and must be positive and finite.

Failed arc steps halve the radius and retry from the last accepted state up to
`max_step_cutbacks`. Each result reports the accepted radius and cutback count;
successful steps scale the next radius with the square root of the configured
target-to-actual Newton iteration ratio, bounded to `[0.5, 2.0]` and capped by
the nominal radius. The visible `arc_length_target_iterations` defaults to 6.
Exhaustion preserves the final inner cause in structured failure detail.

Load control remains an elastic precritical screening path and retains its
`0.95 lambda_cr` guard. Arc-length control may cross that screening guard, but
is still experimental. Retained evidence now includes one shallow-arch limit
point and descending equilibrium branch, not general post-buckling or
bifurcation-branch qualification. Neither control mode is a material-plasticity
model.

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
- Stable incremental length and relative-angle identities preserve short-member
  geometry changes that disappear under direct subtraction of near-equal
  lengths or angles.
- A difficult `0.9 lambda_cr` single-step path fails transparently when cutback
  is disabled, including its last achieved factor, then reaches the same target
  through five accepted substeps when adaptive cutback is enabled.
- Newton iteration limit, residual tolerance, and cutback limit are explicit
  request fields. Invalid or excessive values are rejected before assembly;
  every result step reports iterations, substeps, cutbacks, convergence, and
  achieved load factor.
- Failed steps additionally report a stable machine-readable failure reason and
  optional detail. Cutback exhaustion retains the final inner Newton cause;
  converged steps clear both fields, and legacy results default them to null.
- A 96-step corotational arc-length path crosses the load-control screening
  limit, remains converged, and reports both normalized equilibrium residual and
  spherical constraint error below `1e-7` at every accepted point. A rigidly
  rotated copy preserves each load factor and displacement norm.
- Legacy requests default to `load_control`; linearized kinematics cannot select
  arc length, and invalid radius or load-scale values fail before assembly.
- The workflow executor preserves arc-length control and its per-step constraint
  diagnostics through JSON execution.
- An intentionally oversized one-step radius fails transparently when cutback
  is disabled, then converges with a smaller reported radius when enabled.
- A low explicit target iteration count continuously reduces subsequent
  accepted radii; invalid targets and targets above `max_iterations` are
  rejected before assembly. The engine workflow preserves the target value.
- A two-member shallow arch reaches an interior load-factor maximum, follows
  the descending branch through zero load, and inverts its crown. The retained
  peak matches an independently evaluated pin-jointed shallow-arch reference
  within 2%, while every accepted point satisfies both `1e-7` error gates.
- Subdividing each arch side into 2, 4, and 8 frame elements resolves an earlier
  member-flexural instability branch hidden by the rigid-link-like two-element
  fixture. Linear critical factors converge to within 0.1% between the two
  finest meshes; nonlinear peaks decrease monotonically and differ by less than
  10% between those meshes. Every mesh retains a descending branch through zero
  load while satisfying both `1e-7` gates.
- Explicit fields with the wrong DOF count, non-finite entries, or no
  translational imperfection are rejected rather than falling back to a mode.
- Amplification increases monotonically and reduced equilibrium residuals stay
  below the retained numerical tolerance.
- The default path ends at `0.8 lambda_cr` when no maximum factor is supplied.
- User load-controlled paths at or above `0.95 lambda_cr` are rejected before
  tangent solution; the `0.95` limit is returned as read-only result metadata.
- The workflow executor and agent RPC retain imperfection amplitude, mode index,
  explicit shape, source kind, load-step count, full displacement history, and
  solver provenance.

## Promotion Gaps

- complex-topology and interacting multi-mode continuation references
- problem-scale minimum and maximum radius policies beyond the nominal cap
- bifurcation detection and explicit branch switching
- material yielding, residual stress, and section interaction
- post-critical branch switching and external reference correlation

## Performance Reproduction

On the local macOS release build, paired medium cases (121 nodes, 120 elements,
363 total DOFs, eight load steps) completed with a `106.8162 ms` linearized
median and a `91.0838 ms` corotational median over three repetitions. Both used
`9 MiB` peak RSS. The shared buckling preload dominates this small screening
case, so these figures demonstrate absence of an analytic-tangent regression,
not a general speed advantage for corotational analysis.

The retained large linearized case (2,501 nodes, 2,500 elements, 7,503 total
DOFs) reached a `775.8730 ms` median over three repetitions with `19 MiB` peak
RSS. The corotational large case completed all eight nominal steps with a
`1,086.0335 ms` median and `24 MiB` peak RSS over three repetitions. Benchmark
execution rejects any result whose requested path did not converge.

```text
cd workers/rust
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile medium --repeat 3 \
  --case frame-2d-p-delta-medium --format table
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile medium --repeat 3 \
  --case frame-2d-corotational-medium --format table
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile large --repeat 3 \
  --case frame-2d-p-delta-large --format table
cargo run --release -p kyuubiki-benchmark -- \
  --matrix stability-screening --profile large --repeat 3 \
  --case frame-2d-corotational-large --format table
```
