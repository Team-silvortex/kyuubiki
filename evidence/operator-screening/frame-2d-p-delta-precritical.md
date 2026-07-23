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
- The classic Williams toggle frame adds an external snap-through event
  reference using the published `12.943 in` half span, `0.386 in` rise,
  `0.753 x 0.243 in` section, and `10.3e6 psi` modulus. A ten-element
  corotational path reaches `35.0024 lb`, within 5% of the published analytic
  `34.0392 lb` limit, marks that sample as `limit_point_maximum`, and retains
  negative load increments afterward. The model and analytic limit are
  traceable to [Williams (1964)](https://doi.org/10.1093/qjmam/17.4.451) and
  an [independent nonlinear-frame study](https://dspace.library.uvic.ca/bitstreams/91d6532a-9a66-4ce2-a33c-cbba9829049b/download).
- A pinned Euler column adds an external pitchfork reference at
  `pi^2 EI / L^2 = 986.9604`. Eight- and sixteen-element linear factors are
  `986.9928` and `986.9625`; their nonlinear switched-seed means move from
  approximately `1009.91` to `1000.14`, toward the analytic load. The fine
  transition is labeled `bifurcation_candidate`, has a normalized critical
  eigenvalue below `1e-8`, and produces distinct positive and negative modal
  seeds. Both signed branches complete eight continuation steps with force and
  arc-length errors below `1e-7` and opposite midpoint displacements.
- Two identical, independently supported Euler columns then create an exact
  analytic double eigenvalue. Their first two factors agree with
  `pi^2 EI / L^2` within 0.2% and with each other within `1e-10`. A degenerate
  subspace gauge now augments `m` modal projection equations with `m - 1`
  orthogonal correction columns, preventing an unselected zero mode from
  collapsing every corrected seed onto the same branch. All four individual
  and four pairwise signed probes become distinct and continue for four steps.
  In the physical coordinates of the two columns, the positive probes cover
  left-localized, right-localized, same-direction, and opposite-direction
  families with worst absolute alignment above 0.9. Caller-weighted and
  automatic directions deliberately retain their single combined constraint,
  so nonlinear component ratios remain free to rotate.
- A connected-topology reference joins the two column midpoints with a weak
  axial coupler of stiffness `k = 20`. The symmetric mode leaves the coupler
  unstretched and remains within 0.2% of `pi^2 EI / L^2`; the antisymmetric
  mode stretches it and remains within 0.5% of the weak-coupling Rayleigh
  reference `pi^2 EI / L^2 + 4 k L / pi^2`. Midpoint signs independently
  identify the lower mode as symmetric and the raised mode as antisymmetric.
  The first connected nonlinear transition emits two distinct symmetric
  branches; both complete four continuation steps with force and arc-length
  errors below `1e-7` and terminal symmetric alignment above 0.99.
  Reversing the second-column imperfection retains the antisymmetric invariant
  path and exposes two ordered bifurcation candidates as the tangent inertia
  count rises from one to two negative directions. The secondary signed seeds
  lie within 3% of the antisymmetric Rayleigh reference, remain above 0.99
  aligned with the physical opposite-direction mode, and each complete four
  continuation steps below both `1e-7` error gates.
- Raising only the right-column inertia by 0.5% removes the symmetry invariant
  subspaces. Both connected eigenloads remain within 0.5% of an independent
  two-degree-of-freedom Rayleigh reduction, and each numerical eigenvector has
  above 0.98 absolute midpoint alignment with its externally derived mixed
  direction while both columns retain material participation. The lower and
  upper mixed single-mode transitions emit distinct signed branches and
  complete four continuation steps below both `1e-7` error gates. At the upper
  transition, the two retained tangent eigenvalues are separated: explicit
  pairwise requests remain visible but are rejected before equilibrium
  correction with a degenerate-eigenspace diagnostic. This prevents four
  combinations from collapsing onto one physical branch and being falsely
  reported as distinct.
- Three identical columns with equal-stiffness midpoint links on every pair
  form a connected complete graph. The uniform mode remains within 0.2% of
  `pi^2 EI / L^2`, while the next two factors agree with each other within
  `1e-9` and remain within 0.5% of the graph-Laplacian Rayleigh reference
  `pi^2 EI / L^2 + 6 k L / pi^2`. Both repeated eigenvectors retain at least
  two participating columns and near-zero column-amplitude sum. An
  antisymmetric invariant path reaches the repeated nonlinear transition,
  where two normalized critical modes produce four individual and four
  pairwise signed probes. All eight are distinct and each completes two
  continuation steps with force and arc-length errors below `1e-7`. This is a
  genuinely connected, mixed, degenerate subspace; it does not rely on
  disconnected copies.
- A five-point two-parameter grid independently perturbs the third-column
  inertia and one complete-graph edge stiffness around that repeated point.
  At every grid point, the first three finite-element factors remain within
  0.5% of the closed-form eigenvalues of the corresponding reduced symmetric
  3-by-3 Rayleigh matrix. The repeated factor remains unresolved only at the
  symmetric origin and develops a relative split above `1e-4` under either
  parameter perturbation; the measured split itself agrees with the reduced
  spectrum within 0.5%. This validates the linear spectral unfolding, not yet
  the complete nonlinear two-parameter branch surface. A five-point
  semicircular parameter path around the repeated origin additionally drives
  each nonlinear solve with the overlap-tracked external mixed eigenvector. At
  every point, the selected tangent critical direction retains above 0.9
  alignment with that reduced reference; both signed equilibrium seeds retain
  above 0.98 alignment with the selected tangent mode and complete two
  continuation steps below both `1e-7` error gates. Neighboring external and
  finite-element directions each retain above 0.75 absolute overlap.
- The same triplet now exports a complete arc-length continuation state from a
  selected nonlinear branch: full displacement and displacement-increment
  vectors plus load factor and load-factor increment. That state is corrected
  to equilibrium after each parameter change rather than trusted as-is. A
  retained regression starts on the negative inertia-skew side, passes through
  the exact repeated origin, and exits on the positive side after mode ordering
  exchanges. Both state-seeded solves converge below the `1e-7` equilibrium and
  arc-length error gates, retain above 0.9 alignment at the repeated point, and
  retain above 0.85 alignment with the overlap-selected physical mode after the
  exchange.
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
- Every primary arc-length result now exports `continuation_state`; callers may
  feed it back through the optional request field of the same name. Imported
  states require corotational arc-length control, exact full-DOF vector sizes,
  finite values, and zero constrained components. The solver first corrects the
  imported displacement to equilibrium in the current model and reports
  `continuation_state_correction_norm`, while preserving the prior generalized
  increment for branch orientation. The engine JSON route round-trips and
  executes this state, so headless callers do not need an in-process solver
  shortcut.
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
- A separate topology-isolation reference compares the host arch against that
  same unloaded free branch under directly comparable controls. Dense
  generalized buckling now statically condenses degrees of freedom with no
  geometric work, restores their modal motion afterward, and recomputes the
  full residual. One-mode and six-mode requests retain the same first factor;
  the host and branched spectra agree within `1e-9`.
- Sixteen fixed load-factor corotational equilibria then preserve the host's
  common 27-degree-of-freedom displacement field within a `2e-10` vector norm,
  while load factors agree within `2e-12`. Arc-length step indices are
  intentionally not compared because adding a freely moving degree of freedom
  changes the continuation norm and therefore reparameterizes the same path.
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
- `branch_switch_mode_weights` adds one caller-defined direction across all
  requested modes without enabling an automatic combinatorial search. The
  vector must contain exactly one finite weight per requested mode and combine
  at least two nonzero components. A scale-safe normalization accepts very
  large finite magnitudes without overflowing.
- Three identical independent arches create an exact three-dimensional
  critical subspace. The explicit `[1, 2, -2]` direction is normalized to
  `[1/3, 2/3, -2/3]`; both signed equilibria are distinct and complete their
  requested continuation. Their solved component projections remain visible,
  while the weighted sum reproduces the constrained combined projection within
  `1e-10`. Individual component projections are intentionally not frozen
  during nonlinear equilibrium correction.
- `branch_switch_subspace_sample_count` adds a deterministic automatic fan for
  three or four retained modes. It enumerates projectively unique ternary
  directions, requires at least three active components, canonicalizes the
  first nonzero sign, prioritizes full-dimensional directions, and normalizes
  every vector. The request boundary limits three modes to four samples and
  four modes to sixteen, so automatic coverage cannot grow combinatorially.
- The three-arch fixture retains all four automatic three-mode directions and
  both signs of each direction: six individual probes plus eight fan probes.
  Every automatic equilibrium is distinct, every requested two-step
  continuation completes, and each weighted component projection recomposes
  the constrained direction within `1e-10`.
- `branch_switch_subspace_refinement_levels` optionally adds one or two
  response-driven refinement layers. Each layer compares the signed
  equilibrium, primary-correction, and distinct-branch classifications of
  retained direction families, then samples normalized projective midpoints
  across the nearest changing-response boundaries. Midpoints are sign
  canonicalized and deduplicated against all earlier directions.
- Refinement is hierarchical rather than a fresh global rescan: each solved
  midpoint retains only the parent subintervals whose endpoint response
  classifications still differ. Every adaptive probe reports that parent's
  projective width in `subspace_parent_angle_radians`; retained two-layer
  three-arch evidence requires the second-layer maximum width to be strictly
  smaller than the first-layer maximum. An independent analytic projective
  boundary at 37% of a 90-degree arc is retained inside the changing-response
  bracket for eight refinement levels; its width halves exactly at every level
  and falls below 0.007 radians. This checks angular convergence separately
  from any one finite-element branch response.
- Refinement requires the bounded base fan and adds at most its requested
  sample count per layer. At the maximum four-mode setting this limits the
  complete search to 48 direction families, or 96 probes when both signs are
  selected. A three-arch transition exercises a real changing-response
  boundary and retains its one-layer adaptive probes without invalidating the
  primary path.
- Every probe reports an explicit `origin` (`critical_mode`,
  `pairwise_combination`, `caller_weighted`, `automatic_subspace`, or
  `adaptive_subspace`). Automatic and adaptive probes additionally report
  `subspace_refinement_level`, with the deterministic base fan at level zero.
- The engine JSON route executes a four-step positive and negative switched
  continuation, round-trips it through `SolveFrame2dPDeltaResult`, and preserves
  solver provenance, convergence, error gates, events, inertia, and nested
  displacement histories. This capability is therefore available to headless
  callers rather than only direct solver users.
- A second engine route executes the repeated two-mode twin-arch fixture and
  preserves both mode records, the ordered individual attribution
  `[0, 0, 1, 1]`, four pairwise families, one caller-weighted family, all
  component tables and projections, and all nested continuation state.
- A third engine route executes the full three-mode automatic fan with two
  adaptive layers and preserves its bounded controls, origin, refinement
  level, parent angle, normalized component tables, and solved component
  projections for headless callers.
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

- adaptive parameter-step control and independent qualification of state-seeded multi-parameter surfaces
- external post-critical trajectory correlation for an asymmetric connected frame
- material yielding, residual stress, and section interaction

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
