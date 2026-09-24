# Frame Equation-Balance Reliability

Date: 2026-09-24. Local macOS ARM64, in-process Rust tests.

## Scope And Reproduction

This continues the [support-load isolation round](frame-nonlinear-equilibrium-reliability-20260923.md).
After removing constrained loads from the residual denominator, all free
force and moment equations still shared one infinity-norm scale.

Three public negative controls failed before this change. An independent
axial member with a `1e18` free load and proportionally high stiffness was
added beside a small cantilever. Although the members are disconnected, the
cantilever response changed: sampled displacements were
`0.0033435938576220907` instead of `0.003465951168906793`, and
`-0.001989739645502947` instead of `-0.0020840588409277456`. These are about
3.5% and 4.5% discrepancies. A two-iteration attempt also reported convergence
instead of retaining the uncommitted zero state.

The temporary red-run log is named
`kyuubiki-row-equilibrium-red-20260923.log`; the public regression source
is the reproducible evidence, not a requirement to retain that temporary log.

## Implementation

- Added a solver-private `EquilibriumMetric` shared by load control, arc
  length, parameter continuation and modal branch correction. Engine dispatch,
  operator IDs, task/result schemas and material commit policy are unchanged.
- For equation `i`, let `L = max(abs(load_factor), 1)`. The additional row
  scale is `L * max(abs(reference_i), 1) + sum_j abs(K_ij * u_j)`. Each term
  has that equation's units. Only the free row and its tangent entries supply
  this scale; unrelated force or moment rows do not supply it.
- The reported metric is the maximum of this row-wise ratio and the existing
  global free-load residual. This cannot relax the old acceptance requirement.
- Accumulate row magnitudes as a peak plus a normalized sum, rather than form
  a potentially overflowing dimensional denominator. Reject nonfinite
  products, shapes or mappings and retain cooperative cancellation.
- Before line search, use `abs(u_j) + abs(delta_j)` as a fixed local predictor
  amplitude. Re-evaluate the current residual under that same search metric.
  Every trial uses it unchanged. Final convergence is still checked with the
  current-state metric, not the more permissive predictor metric.
- The metric scans existing sparse rows and adds linear-size vectors. It does
  not add a linear solve, section evaluation or dense matrix conversion. No
  performance improvement or large-scale overhead result is claimed.

## Recovery Regression Adjustment

Simply freezing the zero-state scale penalized initially unloaded equations
and broke large-scale cyclic controls; the fixed predictor metric resolves
that regression without changing solver tolerance.

The old failed-reversal fixtures used an almost straight column and three
iterations. Their first step had been accepted with unresolved small bending
residuals. The replacement failure fixture permits four iterations and uses
a finite imperfection (`0.1` in the four-element headless model, `0.01` in the
16-element public model). It asserts that the plastic preload is equilibrated
at `1e-10`, the reversal is not, the last achieved factor remains `1.3`, and
material state is identical to a clean preload-only replay. No tolerance or
physical-reference comparison was loosened. Iteration-budget semantics and
commit-on-convergence behavior remain unchanged.

## Verification

- Nine new kernel checks cover weak forces/moments, cancellation scales,
  frozen trial/predictor metrics, map ordering, invalid products and cooperative
  cancellation. The six support-load kernel checks remain in the same scope.
- Four new public tests cover independent strong/weak loading, weak moments,
  exhausted budgets and arc-length equilibrium at each achieved factor.
- Three new in-process Rust headless tests cover the real plan/bridge/operator
  route for mixed-scale load/arc paths, cyclic material history and rejection
  of an unbalanced material trial followed by a clean replay.
- Core libraries: **1,184 passed, 7 pre-existing ignored**.
- Adjacent public, headless and engine targets: **106 passed across 16 targets**.
- Extended coupled-Euler, Williams-toggle and review references: **14 passed
  across 3 more targets**. Combined integration coverage is **120 passed**.
- Optimized build: **53 passed** (15 metric kernels, 8 public equilibrium,
  12 public material and 18 headless cases), including all 16 new cases.
- Executed scoped profile: **85 test executions across 4 commands**, all
  passed. These overlap the suites above and are not additional unique tests.
- Strict solver all-target and both headless-route Clippy checks passed with
  warnings denied. Targeted formatting and `git diff --check` passed.
- Operator-profile, coverage-tensor, organization and documentation gates
  passed. Source/document caps remain **800/2,000 lines**, tracked debt zero.
  The tensor still has **4 maturity gaps, 16 evidence-grade gaps and 11 P0
  gaps**; overall Daji qualification remains **blocked**.

Re-run the bounded profile:

```text
./scripts/kyuubiki check-operator-validation --execute --profile frame-equation-balance-local-reliability --out tmp/frame-equation-balance-local.json
```

## Boundaries

This is bounded local equation-balance validation, not a general nonlinear error certificate or structural qualification.

The unit-valued and load-factor floors are intentionally retained. The row
scale is a local tangent-based convergence guard, not a proof of forward
displacement accuracy, exact nonlinear backward error, mesh convergence,
constitutive accuracy or all-unit/reference-load invariance. Strongly coupled
ill-conditioned and near-singular systems still need independent references.
The independent extra DOF changes arc-length parameterization, so the arc
comparison is made against a load-controlled solve at the actual factor.

No installed GUI, remote Linux, durable restart, distributed scheduling,
large-model benchmark or overall Daji qualification is claimed. Temporary
results stay outside tracked release artifacts. No package version, commit,
push or installed application is changed by this round.
