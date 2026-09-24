# Nonlinear Spring And Gap Contact Committed-State Recovery

Date: 2026-09-24. Local macOS ARM64, in-process Rust tests.

This follows the [finite-range regression](nonlinear-spring-range-reliability-20260924.md).
It changes finite non-convergence handling, not the underlying spring or
penalty law, absolute residual tolerance, operator IDs or engine dispatch.

## Reproduced Failures

Four public negative controls failed before the change:

- A linear one-DOF spring with one allowed Newton correction reached its
  exact solution but still returned `converged=false` because the corrected
  state was not evaluated.
- For `F = u + u^3`, load `2` and one correction from zero, the returned
  displacement was `2` with a reported trial residual of `2`. The actual
  residual magnitude at that trial is `8`.
- A four-step contact path did not consistently commit completed load steps
  or preserve their state after contact activation exhausted the budget.
- Two corrections were enough to resolve the retained penalty transition,
  but the final correction was not checked before declaring failure.

Two benchmark controls also failed: finite non-convergence was marked as a
successful benchmark, and successful spring results lacked iteration/residual
diagnostics in the benchmark report.

## Result Contract

Both operators now share a solver-private load-path loop. It evaluates the
initial trial and every correction, including the last permitted one. The
iteration budget counts Newton corrections, not residual evaluations. An
already equilibrated step reports zero corrections. No extra linear solve is
permitted after the budget is exhausted.

Only converged steps commit physical state. On finite budget exhaustion,
nodes, element response, contact activation/force and summary fields are
derived from the last committed displacement. Failure on the first step
returns the initial zero-load state. The failed target remains in `steps`.

- `converged` means the entire requested path reached its final target.
- New `achieved_load_factor` identifies the load factor of returned physical
  state: `0` for no committed step, a fraction for partial completion, `1`
  for completion. It is optional in deserialization; absent legacy metadata
  remains `None`/omitted rather than inventing a committed load factor.
- Top-level `residual_norm` belongs to returned state at that achieved load.
  It can be zero even when `converged=false`; it is not full-load acceptance.
- Each `steps[].residual_norm` belongs to that step's last evaluated trial at
  `steps[].load_factor`. A failed trial's residual is retained without
  publishing its unequilibrated displacement as the solution.

There are two reusable displacement state buffers, not a full node snapshot
for every load step. Newton corrections and linear-system workspaces still
exist separately. State copying is cooperatively cancellable. Sparse reduction
is skipped when residual evaluation already determines convergence or budget
exhaustion. The tridiagonal path and sparse fallback remain unchanged.

Cancellation, invalid arithmetic and linear-solver failures still return
errors; rollback does not turn them into successful partial results. The
benchmark adapter now rejects finite non-convergence and reports the failed
target, achieved load, residual and corrections in its error. Successful runs
retain correction totals and residual diagnostics.

## Verification

The regression suite covers one-correction completion; positive/negative
loading; first-step failure; hardening and penalty rollback with both zero and
nonzero committed residual; failed contact activation; cancellation after
committed steps; clean replay with a larger budget; and unloaded paths.
Two deliberately loose-tolerance fixtures verify that a nonzero committed
residual is retained accurately; they are bookkeeping controls, not accuracy
qualification. A 130-element chain checks cancellation during committed-state
copying and subsequent clean replay.

Rust headless execution uses the actual plan/bridge/operator route. Real
workflow tests preserve the partial-state metadata and compare successful
replay with direct engine execution. Protocol tests cover old JSON plus new
zero/partial/full load factors. Benchmark controls exercise the real runner.

- Nineteen new tests: 10 public recovery, 3 headless route, 2 protocol, 2 real
  workflow and 2 benchmark-runner controls.
- Executed bounded profile: **74 passed across 5 commands / 16 test suites**,
  including the retained analytical, convergence and finite-range regressions.
  The 19 new tests are part of this total, not an additional 19 executions.
- Core protocol, solver, engine and headless SDK libraries: **1,184 passed,
  7 pre-existing ignored**.
- Optimized release build: **36 passed** across the public recovery/range,
  protocol, headless, workflow and benchmark targets. This includes all 19 new
  tests and 17 retained range tests; it is repeat validation, not extra unique
  coverage.
- Strict solver/protocol/benchmark all-target Clippy and the two modified
  headless/workflow target checks passed with warnings denied.
- Targeted formatting, whitespace, operator-profile, coverage-tensor,
  organization and documentation gates passed. Source/document caps remain
  **800/2,000 lines**, tracked debt zero.
- The tensor still has **4 maturity gaps, 16 evidence-grade gaps and 11 P0
  gaps**; overall qualification remains **blocked**. This local scope does not
  promote any broad physics or operational qualification.

Re-run the bounded profile:

```text
./scripts/kyuubiki check-operator-validation --execute --profile nonlinear-spring-committed-recovery --out tmp/nonlinear-spring-committed-recovery.json
```

## Boundaries

This is bounded local spring/contact committed-state validation, not durable restart, general nonlinear convergence or industrial contact qualification.

Callers must inspect `converged`, not just a small top-level residual or a
completed workflow node. Generic workflows continue to carry partial solver
results for caller-defined policy; they do not automatically retry or block
every downstream operator. The new contract provides in-process committed
state, not a serialized resume token, process-restart recovery, adaptive load
cutbacks, line search, friction/contact search or bifurcation treatment.

No installed GUI, remote Linux, distributed run or large-model performance
qualification is claimed. No app package or version is changed by this round.
