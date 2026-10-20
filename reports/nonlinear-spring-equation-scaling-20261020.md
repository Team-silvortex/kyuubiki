# Nonlinear spring equation scaling

Date: 2026-10-20. Version: daji 3.6.0.

## Reproduced failures

Three new kernel tests and one public-operator test failed before this repair.

With `s = 2^1023`, the tridiagonal system
`A = s * [[1, -0.5], [-0.5, 1]]`, `b = s * [1.5, 1.5]` has the exact finite
solution `[3, 3]`. Nevertheless, forward elimination produces `2.25*s`,
which overflows before division by the final pivot.

The public mechanical model is a four-node, three-spring chain with both ends
fixed, each spring stiffness `0.5*s`, and each free node carrying load `1.5*s`.
Both free displacements are 3. The two end springs carry forces `+1.5*s` and
`-1.5*s`; the middle spring is unloaded. All requested inputs and physical
outputs are representable, but the old implementation reported
`tridiagonal elimination diverged`.

A second system, `A = s * [[1, 1], [1, -1]]`, `b = s * [1, 0]`, has solution
`[0.5, 0.5]` but overflows its Schur pivot. At the other end of the range,
using `s = 2 * minimum_subnormal` in the first system rounds the eliminated
equation and fails backward-error validation.

## Repair

If a row's maximum coefficient has a binary exponent outside `[-500, 500]`,
equilibrate each equation by a power of two before factorization. This is
algebraically the same system: `R*A*x = R*b`. Unknowns, physical units,
operator laws, engine dispatch and result schemas do not change.

Radix-power equilibration is a standard numerical technique; see
[LAPACK DGEEQUB](https://www.netlib.org/lapack/explore-html/df/d0a/group__geequb_gaef3a3015d58122ab3d4d018422b93b90.html).
This implementation performs bounded row scaling only, not LAPACK's full
row/column algorithm, and introduces no external dependency.

- The existing ordinary-range factor path is retained.
- Scaling retains the tridiagonal structure and O(n) storage/work.
- Every coefficient and each new right-hand side must survive an exact
  reverse scaling check. Overflow, flushing a nonzero value to zero, and
  partial precision loss all fail explicitly.
- Store row exponents with reusable factors. A failed or cancelled load
  does not mutate those factors.
- Reuse adjacent-row pivoting for the normalized matrix when needed.
- Validate the final solution against the original, unscaled coefficients
  and loads with exponent-scaled, row-local backward error.
- Check cancellation during coefficient and load normalization; do not
  turn cancellation into a partial success or invoke a dense fallback.

Optional pivot/equilibration state is boxed so the shared prepared-solver enum
does not grow a large inline variant. Ordinary solves do not allocate either
optional state.

## Verification

The same selected regression set passed on macOS AArch64 and Linux x86_64,
with Rust 1.88.0 and offline locked dependencies:

| Test group | macOS | Linux |
| --- | ---: | ---: |
| Tridiagonal-related solver unit tests | 51 | 51 |
| Solver integration tests | 156 | 156 |
| Engine workflow and Rust headless SDK integration tests | 27 | 27 |
| Spring-recovery benchmark classification controls | 4 | 4 |
| Total selected tests | 238 | 238 |

This increment adds 13 tests: nine kernel tests, two public solver tests,
one workflow test and one headless SDK test. The integration set retains
mechanical, acoustic, advection-diffusion, magnetostatic and thermal controls.
These are selected regressions, not the full workspace suite or a throughput
measurement.

Local targeted Clippy passed with `-D warnings` for the solver, engine and CLI
libraries and the affected public-solver, workflow and headless test targets.
Workspace formatting and `git diff --check` passed. Validation metadata still
has 59 profiles. Coverage-tensor structure and command checks pass, while its
release-readiness gate remains blocked with 18 P0 items; no evidence grade or
maturity is promoted.

New tests cover large eliminated loads, overflowing Schur pivots, subnormal
coefficients, mixed row scales, pivoted repeated loads, rejected lossy scaling,
original-equation validation, genuine rank loss and cancellation/replay.
The public chain runs through borrowed/owned APIs, engine workflow dispatch
and Rust headless plan execution with JSON result serialization.

Focused reproduction from `workers/rust`:

```text
cargo test --offline --locked -p kyuubiki-solver --lib tridiagonal
cargo test --offline --locked -p kyuubiki-solver --test nonlinear_spring_equation_scaling
cargo test --offline --locked -p kyuubiki-engine --test nonlinear_spring_recovery_workflow
cargo test --offline --locked -p kyuubiki-cli --test nonlinear_spring_range_operator
```

## Limits

This is bounded local spring/contact numerical-range validation, not general nonlinear or industrial contact qualification.

Equilibration is not a conditioning, stability or convergence certificate.
It does not provide arbitrary-range arithmetic: a row whose coefficient or
load ratios cannot be represented after scaling is rejected, not rounded into
a different system. Extreme right-hand-side growth in an ordinary-coefficient
system and cancellation inside a factorization are not generally solved here.

The public physical example exercises the linear limit of the nonlinear spring
operator. This does not establish extreme-range coverage for every operator
using the kernel or every surrounding assembly/result-validation path.
The 2,048-row control checks cancellation, factor reuse and absence of dense
fallback, not large-scale throughput. No maturity promotion, new protocol,
application package or runtime deployment is part of this repair.
