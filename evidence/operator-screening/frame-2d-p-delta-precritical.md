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

Every step reports its signed load-factor increment. A sign reversal between
accepted increments marks the preceding equilibrium point as
`limit_point_maximum` or `limit_point_minimum`; failed and numerically zero
increments do not create events.

Load control remains an elastic precritical screening path and retains its
`0.95 lambda_cr` guard. Arc-length control may cross that screening guard, but
is still experimental. Retained evidence now includes shallow-arch limit
points, descending and rising equilibrium segments, and opt-in
critical-mode-constrained branch probes with switched-branch continuation.
These remain screening paths rather than externally qualified post-buckling
branches. Neither control mode is a material-plasticity model.

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
- Both the two-member and segmented shallow arches mark their sampled peak as
  `limit_point_maximum`. The monotonic portal path emits no path event, and the
  engine JSON route preserves signed increments and null events.
- Every converged arc-length point up to 256 reduced DOFs reports symmetric
  tangent inertia. Stable starting points are `positive_definite` with zero
  negative pivots, while shallow-arch descending branches expose at least one
  negative direction as `indefinite`. Zero or numerically degenerate pivots are
  distinguished as `near_singular`.
- The dense inertia observer is deliberately capped at 256 reduced DOFs. Larger
  paths continue solving but report `unassessed_size_limit` with null counts,
  rather than presenting a guessed stability classification.
- Consecutive assessed points expose `tangent_negative_pivot_delta`. A nonzero
  change without a nearby load-increment reversal emits
  `bifurcation_candidate`; changes sampled in a limit-point neighborhood retain
  the limit-point event and are not double-labeled. The candidate is a bracket
  for later branch-switch confirmation, not a proof of a bifurcation.
- The 2/4/8-element-per-side shallow-arch sequence emits one pre-peak candidate
  at load factors `5.5965`, `5.4051`, and `5.3595`, respectively, each with one
  newly negative tangent direction. The two finest candidate factors differ by
  less than 1%, while the rigid-link-like two-member fixture retains only its
  load-reversal limit point.
- Every assessed inertia transition up to 128 reduced DOFs additionally extracts
  the smallest-absolute symmetric tangent eigenpair. The returned shape uses the
  complete global DOF layout, is unit-normalized with constrained entries fixed
  to zero, and has a deterministic sign. Segmented-arch candidate eigenvalues
  are negative and their matrix-scale-normalized residuals stay below `5e-8`.
- `branch_switch_mode_count` requests one to four absolute-eigenvalue-ordered
  tangent modes at each assessed transition. The modes are extracted in one
  Jacobi decomposition, unit-normalized, sign-canonicalized, and returned with
  index, normalized eigenvalue, residual, and full constrained-DOF shape.
  Explicit counts outside `[1, 4]` or counts without branch switching are
  rejected before assembly.
- The legacy singular `tangent_critical_eigenvalue`,
  `tangent_critical_mode_residual`, and `tangent_critical_mode` fields remain an
  exact view of ordered mode zero. New callers can consume
  `tangent_critical_modes` without breaking old result readers.
- Critical-mode extraction uses cyclic Jacobi sweeps only at inertia changes,
  rather than an eigen solve at every continuation point. Models from 129 to 256
  reduced DOFs retain inertia classification and candidate events but leave the
  critical-mode fields null; larger models leave both dense diagnostics
  unassessed while continuation remains available.
- Candidate intervals are refined through configurable equilibrium-corrected
  load bisection (`tangent_transition_refinement_steps`, default 12, maximum
  20). Every midpoint is returned to nonlinear equilibrium before its inertia
  selects the retained half; failed correction or a third inertia state leaves
  the original coarse candidate intact instead of fabricating a bracket.
- Twelve refinements contract each segmented-arch interval by more than 4,000.
  The selected critical loads are approximately `4.8576`, `4.2114`, and
  `4.0549` for 2/4/8 elements per side. They decrease monotonically, the two
  finest differ by less than 5%, and each selected normalized eigenvalue has
  absolute magnitude below `1e-8`.
- Optional `branch_switch` selection accepts `positive`, `negative`, or `both`
  only with corotational arc-length control and a positive finite
  `branch_switch_amplitude`. The default is `disabled`, so existing paths do
  not pay for dense branch diagnostics or change their accepted states.
- At the refined four-element-per-side member-instability transition, positive
  and negative `0.005` critical-mode constraints each converge to a distinct
  equilibrium state. Each candidate is cross-checked against a separately
  corrected primary-path equilibrium at the same solved load; only a finite
  separation of at least half the requested amplitude is labeled
  `distinct_branch`. Both report force residual and normalized modal-constraint
  error below `1e-7`, along with solved load factor, iterations, projection,
  displacement distances, and the full displacement vector.
- Branch probes solve the bordered tangent system for displacement and load
  factor together. Backtracking must reduce the larger of the equilibrium and
  modal-constraint errors; failure remains a structured probe result and never
  replaces or truncates the primary arc-length history.
- `branch_continuation_steps` optionally continues each distinct seed with the
  same spherical arc-length single-step kernel, up to 64 requested points. Each
  branch starts with the generalized direction from the refined critical state
  to its equilibrated seed, then owns independent radius adaptation, cutbacks,
  convergence status, and failure detail.
- `branch_continuation_radius` optionally replaces the inherited primary-path
  radius for switched paths. It must be positive and finite and is rejected
  unless at least one continuation step is requested; each accepted step still
  reports the radius actually used after any adaptive cutback.
- `branch_continuation_min_radius_ratio` optionally sets a dimensionless lower
  bound relative to the visible nominal switched-branch radius. Values must be
  finite and in `(0, 1]` and require requested continuation. Successful
  adaptation clamps at the bound; failed-step cutbacks may reach it exactly but
  never cross it, and an attempted further reduction reports
  `increment_too_small` with both current and minimum radii.
- The retained four-element-per-side arch continues both positive and negative
  seeds for the full 64-point request limit. Every switched point satisfies
  force and arc-length errors below `1e-7`; load factors decrease monotonically
  from approximately `4.21` to `2.85`, both paths retain one negative tangent
  direction without false events, and the two terminal states remain separated.
  The entire primary load and displacement history still matches the probe-only
  run to `1e-11` relative tolerance.
- Repeating that fixture with an explicit `0.02` switched-branch radius carries
  both directions through a physical `limit_point_minimum` at step 44 and load
  factor approximately `-1.473`, then onto a rising segment ending near `0.852`.
  All 128 switched points remain converged with force and arc-length errors
  below `1e-7`; a `0.25` minimum-radius ratio keeps every reported radius in the
  problem-scale interval `[0.005, 0.02]`.
- Switched steps expose the same limit-point and tangent-inertia transition
  vocabulary as the primary path. A retained synthetic reversal confirms that
  a coincident inertia change remains a limit point rather than a false
  bifurcation candidate; the 64-point physical branches confirm the negative
  case by emitting no event on monotonic, constant-inertia paths.
- A rigidly rotated segmented arch preserves switched-branch seed vectors after
  vector rotation, eight-step load histories, complete displacement vectors,
  event fields, and tangent negative-pivot counts. Branch matching is based on
  the transformed seed rather than the arbitrary sign of the normalized mode.
- A complex-topology fixture adds an unloaded cantilever branch at the crown of
  the eight-member arch. Its first inertia transition retains two distinct
  equilibrium seeds and completes 16 switched steps in each direction with
  both error gates below `1e-7`. An arbitrary rigid rotation preserves the
  critical factor, 30-DOF seed and step vectors, load histories, events, and
  inertia counts.
- The branched fixture also encounters a later transition whose positive seed
  is not distinct. That branch reports a local continuation failure with no
  fabricated steps, while the opposite branch and complete primary path remain
  available and converged.
- A retained pair of identical, independently supported eight-member arches
  creates an exact two-dimensional repeated critical subspace. Both normalized
  eigenvalues are approximately `4.50e-11`, both residuals stay below `1e-8`,
  and the two returned shapes are orthogonal. Mode indices zero and one each
  generate positive and negative probes, yielding four distinct branch families
  that each complete their requested switched continuation.
- `branch_switch_pairwise_combinations` is an explicit opt-in layered on a
  requested mode count of at least two. It retains the individual probes and
  adds normalized `mode_i + mode_j` and `mode_i - mode_j` directions for every
  pair; disabled/default requests keep the prior cost and probe count.
- Each combined probe exposes `mode_components` with source mode index,
  eigenvalue, and normalized weight. The singular `mode_index` remains the
  first-component compatibility alias while `mode_eigenvalue` is null rather
  than inventing one eigenvalue for a mixed direction.
- The twin-arch fixture therefore produces eight distinct probes: four
  individual and four pairwise. Every pairwise seed and its requested
  continuation converge. The returned component projections match the signed
  `0.005 / sqrt(2)` targets within `1e-8`, proving that the corrected
  equilibrium follows the selected sum or difference rather than only carrying
  a combination label.
- The engine JSON route executes a four-step positive and negative switched
  continuation, round-trips it through `SolveFrame2dPDeltaResult`, and preserves
  solver provenance, convergence, error gates, events, inertia, and nested
  displacement histories. This capability is therefore available to headless
  callers rather than only direct solver users.
- A second engine route executes the repeated two-mode twin-arch fixture and
  preserves both mode records, the ordered individual attribution
  `[0, 0, 1, 1]`, four pairwise component tables and projections, and all nested
  continuation state.
- A branch assembly or continuation failure is contained in that probe through
  `continuation_converged` and `continuation_failure_detail`. It does not turn a
  valid primary path or the opposite branch into a top-level solver error.
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

- external complex-topology switched-branch reference events
- adaptive three-or-more-mode subspace sampling beyond pairwise directions
- external coupled-topology correlation for interacting pairwise modes
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
