# Nonlinear Spring And Gap Contact Range Reliability

Date: 2026-09-24. Local macOS ARM64, in-process Rust tests.

## Reproduced Failures

Seven public regression tests failed against the previous implementation:

- With `k = 1`, `c = 0` and load `+/-1e200`, unweighted displacement powers
  overflowed before multiplication by zero. A maximum-residual fold ignored
  `NaN`, so the spring and gap-contact paths could claim convergence with
  non-finite element results.
- With `k = 1`, `c = 1e-300` and load `+/-2e150`, the exact response
  `u = +/-1e150`, tangent `4` is representable, but forming `u^3` failed.
- With `c = 1e308` and zero extension, forming `3*c` produced a non-finite
  tangent despite the finite linear response.
- A one-iteration trial could return infinite element force/tangent, while
  two parallel `1e308` springs at zero load could bypass linear validation
  with an overflowing assembled tangent.
- Assembly and result stages did not observe cancellation requests.

These extreme inputs are numerical boundary fixtures, not physical operating
ranges or material-validity claims. The reproducible evidence is the checked-in
test source; temporary logs and generated profile output are not release data.

## Fix And Contract

The solver now shares its spring response and sparse assembly between
`solve.nonlinear_spring_1d` and `solve.contact_gap_1d`. It evaluates the weighted
term `(c*u)*u` before forming cubic force and tangent. It checks element and
contact responses, accumulated internal forces and sparse tangent entries,
free residuals, displacement updates and result summaries for finite values.
Assembly validation runs even if convergence would otherwise skip the linear
solver. Final result construction uses the same checked constitutive helpers,
including when the iteration budget is exhausted.

Constrained node loads are excluded before residual subtraction; they cannot
overflow an equation that is removed by constraints. Absolute residual
tolerance, load stepping, the tridiagonal fast path and general sparse fallback
are unchanged. No engine dispatch, operator ID, task/result schema or worker
SDK boundary changed. The unused external-load copies were removed. Added
validation scans existing sparse entries and vectors without dense conversion.

Cooperative checkpoints cover assembly, sparse validation, residual evaluation,
Newton updates, node/element result construction and summaries. Cancellation
returns an error, not a partial successful result. Clean in-process replay is
tested after cancellation or numerical rejection.

## Verification

- Twelve new public tests cover both borrowed/owned entry points, positive and
  negative loading, inactive/active contact, weighted cubic evaluation, zero
  load, final-trial overflow, support loads, penalty and assembly overflow,
  cancellation at stage entry and inside a 130-element chain, and clean replay.
- Five new Rust headless tests build execution plans, resolve the official
  bridge and execute the actual engine operator. They check finite JSON output
  and error propagation before serialization, including replay after failure.
- Public solver and retained analytic/refinement/convergence targets:
  **48 passed across 9 targets**.
- Core protocol, solver, engine and headless SDK libraries: **1,184 passed,
  7 pre-existing ignored**.
- Executed bounded profile: **55 test executions across 3 commands**, all
  passed (48 public tests, 5 headless tests and 2 real workflow tests). These
  overlap the counts above; the new regression set has 17 unique tests.
- Optimized release build: **all 17 new public/headless tests passed**. This
  repeats the same regression set rather than increasing unique coverage.
- Strict solver all-target and new headless-target Clippy checks passed with
  warnings denied. Targeted formatting and whitespace checks passed.
- Operator-profile, coverage-tensor, organization and documentation gates
  passed. Source/document limits remain **800/2,000 lines**, tracked debt zero.
  The tensor retains **4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps**;
  overall qualification remains **blocked**.

Re-run the scoped profile:

```text
./scripts/kyuubiki check-operator-validation --execute --profile nonlinear-spring-range-local-reliability --out tmp/nonlinear-spring-range-local.json
```

## Boundaries

This is bounded local spring/contact numerical-range validation, not general nonlinear or industrial contact qualification.

The original absolute tolerance and its floor remain, so arbitrary unit/load
scaling is not qualified. A non-finite Newton trial fails closed rather than
adding a new line-search, continuation or automatic rescaling algorithm.
Finite non-converged trials still follow the existing result contract; their
residual diagnostic is the last evaluated iteration, not a newly evaluated
post-update equilibrium certificate. Softening, bifurcation, rollback to a
committed load step and general topology stability remain separate work.

Input cloning/validation and engine serialization are not claimed to have
complete cancellation coverage. No installed GUI, remote Linux, durable
restart, distributed scheduling, large-scale benchmark or broader Daji
qualification is claimed. This round does not repackage or install the app.

Follow-up: the [committed-state recovery round](nonlinear-spring-recovery-20260924.md)
supersedes the finite non-converged trial limitation above. It adds explicit
achieved-load metadata and verifies the final permitted correction before
commit or rollback. The finite-range rejection controls remain active.
