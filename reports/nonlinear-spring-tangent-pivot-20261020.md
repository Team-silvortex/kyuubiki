# Nonlinear spring tangent pivot and residual validation

Date: 2026-10-20. Version: daji 3.6.0.

## Reproduced failures

A three-node gap-contact model changed outcome when only the free-node
numbering changed. The original order reported a singular tridiagonal pivot;
reversing the free nodes reached the manufactured equilibrium.

- Node 0 is fixed, node 1 carries force 2, node 2 is unloaded.
- Spring 0-1 has `f(u) = 2*u - u^3`; spring 1-2 has linear stiffness 1.
- The contact at node 1 has gap 1 and penalty stiffness 5.8125.
- At the first Newton trial both free displacements equal 1. Contact is just
  touching and inactive; the reduced tangent is `[[0,-1],[-1,1]]`.
- This matrix has determinant -1, so a zero leading pivot is not rank loss.
- At the final equilibrium both displacements equal 1.25. Spring force is
  0.546875 and contact force is 1.453125. The reduced tangent is positive
  definite there, although the spring element tangent is negative.

Three initial kernel tests also failed for invertible zero leading/interior
pivots and repeated row exchanges. A genuine-rank-loss control passed.

During validation of the repair, three residual-check negative controls
exposed another defect. Overflow in the error denominator made wrong solutions
report zero error; overflow in one large row masked another small equation;
and `f64::max` discarded a NaN relative residual from overflowing products.
All three returned zero before the residual guard repair.

## Changes

The existing no-swap path is retained. When a pivot is unusable, or its
subdiagonal is larger, refactor once using adjacent-row partial pivoting.
Store only the diagonal, first/second superdiagonals, multipliers and swap
flags. Both factorization and solve remain O(n); there is no dense fallback,
automatic regularization, tolerance relaxation or catch-all retry.

The numerical approach follows the standard tridiagonal partial-pivoting
structure described by [LAPACK DGTSV](https://www.netlib.org/lapack/explore-html/d6/ddb/group__gtsv_ga6b06a11b8e2dced10bc9cd3c20a09484.html).
This is a native Rust implementation using the existing cancellation and
original-equation validation contracts, not a new LAPACK dependency.

Both solve paths validate against saved original coefficients. When ordinary
residual arithmetic overflows, normalize each of the four row terms
(`b`, diagonal product, lower product, upper product) by their largest binary
exponent before forming products/sums. Use row-local backward error on this
fallback, rather than an infinite global roundoff floor. Subnormal values and
zero terms are handled explicitly. Normal-range error policy is unchanged.
Both residual passes are cancellable in bounded chunks.

Changes stay in the shared numerical kernel. Operator laws, engine dispatch,
request/result schema and nonlinear convergence budgets are unchanged.
Newton nonconvergence still retains the committed physical state; cancellation
remains an error rather than a partial successful result.

## Verification

The new public model passes in both node orders, at force scales `2^-200`, 1,
and `2^200`, through borrowed and owned entry points. Kernel coverage includes
manufactured symmetric/asymmetric systems cross-checked with dense LU,
repeated right-hand sides, a 4,096-DOF forest without dense factorization,
rank-loss rejection, ordinary/overflow residual checks and cancellation/replay.

The same selected regression set passed on macOS AArch64 and Linux x86_64,
using Rust 1.88.0 and the offline locked dependency set:

| Test group | macOS | Linux |
| --- | ---: | ---: |
| Tridiagonal-related solver unit tests | 42 | 42 |
| Solver integration tests | 154 | 154 |
| Engine workflow and Rust headless SDK integration tests | 25 | 25 |
| Spring-recovery benchmark classification controls | 4 | 4 |
| Total selected tests | 225 | 225 |

There are 18 new tests: 13 kernel tests, three public solver tests, one engine
workflow test and one headless SDK test. The integration set includes existing
nonlinear spring/contact, linear/dynamic/harmonic spring, thermal bar, acoustic
bar, advection-diffusion bar and magnetostatic bar regressions. These are
selected regressions, not the full workspace suite or a performance benchmark.

Local targeted Clippy passed with `-D warnings` for the solver library and
new public solver test, engine recovery workflow, and headless range test.
Workspace formatting, explicit changed-file formatting and `git diff --check`
passed. The operator-validation metadata check retains 59 profiles.
Coverage-tensor structure and command checks pass, but release readiness
remains blocked with 18 P0 items; no evidence-grade promotion is claimed.

Reproduce the focused new paths from `workers/rust`:

```text
cargo test --offline --locked -p kyuubiki-solver --lib tridiagonal
cargo test --offline --locked -p kyuubiki-solver --test nonlinear_spring_tangent_pivot
cargo test --offline --locked -p kyuubiki-engine --test nonlinear_spring_recovery_workflow
cargo test --offline --locked -p kyuubiki-cli --test nonlinear_spring_range_operator
```

## Limits

This is bounded local spring/contact numerical-range validation, not general nonlinear or industrial contact qualification.

The test's intermediate tangent is indefinite but nonsingular. Allowing that
linear Newton correction is not a certificate of stable equilibrium or a
guarantee of crossing arbitrary limit points. Truly singular, unusably small,
or nonfinite factors still fail; this does not add arc-length continuation,
durable recovery of a failed factorization, or arbitrary-condition accuracy.
The residual fallback does not make factorization/substitution range-unlimited.

The 4,096-DOF control checks bounded storage, cancellation and absence of dense
fallback, not 1M throughput. No new operator, service deployment, application
package, compatibility schema, or maturity promotion is part of this change.
