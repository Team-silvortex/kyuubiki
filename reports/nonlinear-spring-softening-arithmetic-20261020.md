# Nonlinear spring softening arithmetic reliability

Date: 2026-10-20. Version: daji 3.6.0.

## Scope

Improve evaluation of `f(u) = k*u + c*u^3` and `t(u) = k + 3*c*u^2`
for the existing nonlinear spring and gap-contact operators. Keep the engine,
request/result schema, Newton budget, tolerance, backtracking and committed-state
contracts unchanged. No installed application or service is replaced.

This is bounded local spring/contact numerical-range validation, not general nonlinear or industrial contact qualification.

`converged` denotes force equilibrium, not tangent/global stability. An element
with negative tangent is not automatically an unstable constrained system:
other elements or active contact may supply positive stiffness. This change
does not add a general stability certificate or post-limit continuation method.

## Reproduced defects

Before the production fix, three new constitutive tests and two new complete
contact solves failed. The true-overflow negative control and ordinary stable
softening public-entry control passed.

With `B = 2^1022`, all inputs in these examples are finite:

| Case | k/B | c/B | u | Exact f/B | Exact t/B | Old failure |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Tangent cancellation | 3 | -1.5 | 1 | 1.5 | -1.5 | `3*c*u^2` overflowed before adding k |
| Force cancellation | 2.5 | -0.25 | 2 | 3 | -0.5 | `k*u` overflowed before adding the cubic force |

These are not merely inaccessible material evaluation points. Add unilateral
contacts `(gap, penalty/B)` of `(0.5, 2)` and `(1.5, 1)`, respectively.
Loads `2.5*B` and `3.5*B` give these equilibria with a positive reduced tangent
of `0.5*B`. The zero-displacement initial states have inactive contacts, and
ordinary Newton/backtracking reaches the equilibria after the fix.

Also, with `k = 1`, `c = -1`, and `u = 1+d`, use the independent factored
reference `f = -d*(2+3*d+d^2)`. Separately rounded polynomial terms lost
significant relative accuracy in the small remaining force. Signed tests cover
`d = 2^-26, 2^-27, 2^-30, 2^-40, 2^-50` and force scales `2^-500, 1, 2^500`.

## Implementation

- Form the force in Horner form, `(k + (c*u)*u)*u`, with a fused multiply-add
  for the inner sum; cancel softening contributions before the final multiply.
- Use a fused multiply-add for `k + 3*((c*u)*u)` so the cubic tangent term need
  not be separately representable when its sum with k is finite.
- Preserve finite-value checks on constitutive values, assembled internal
  forces, assembled tangent, displacement, residuals and contact output.
- Keep borrowed/owned spring/contact paths on the same implementation.
- Document that equilibrium convergence and an individual tangent sign are
  not stability certificates; no serialized field is added.

The minimum-subnormal tolerance regression previously recomputed the force
with the old rounding order. It now checks the residual against the actual
returned element force exactly, and independently compares that force with
the cubic polynomial within its floating-point rounding scale. The requested
tolerance is not relaxed.

## Validation

Both platforms used Rust 1.88.0 and existing offline build caches: local
macOS AArch64 and the lab's isolated Linux x86_64 source checkout. Only the six
changed Rust source/test files were synced; deployed services were untouched.

| Selected group | macOS | Linux |
| --- | ---: | ---: |
| Constitutive unit tests | 4 passed | 4 passed |
| Spring/contact and mechanical integration controls | 96 passed | 96 passed |
| Engine workflow tests | 10 passed | 10 passed |
| Rust headless plan-to-engine tests | 13 passed | 13 passed |
| Benchmark correctness controls | 4 passed | 4 passed |
| Total distinct selected tests per platform | 127 passed | 127 passed |

The 96 solver integration tests include four new softening arithmetic tests
and the previous 92 controls. The constitutive unit tests, workflow test and
headless JSON round-trip test bring the new-test count to ten.

Local Clippy with `-D warnings` passed for the solver library, changed solver
integration targets, and changed engine/CLI integration targets. Workspace
format and diff-whitespace checks passed. Touched source/config files remain
below 800 lines.

Validation-profile structure passed for 59 profiles (`executed=false`; this
check does not run every profile's commands). Tensor structure/command checks
passed with 13 modules, 11 paradigms and zero structural gaps. Overall readiness
remains blocked: 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.

Reproduction commands, from `workers/rust`:

```text
cargo test --offline --locked -p kyuubiki-solver --lib nonlinear_spring_1d::response_tests
cargo test --offline --locked -p kyuubiki-solver --test nonlinear_spring_softening_arithmetic --test nonlinear_spring_tolerance --test nonlinear_spring_range_reliability
cargo test --offline --locked -p kyuubiki-engine --test nonlinear_spring_recovery_workflow
cargo test --offline --locked -p kyuubiki-cli --test nonlinear_spring_range_operator
cargo test --offline --locked -p kyuubiki-benchmark spring_recovery
```

## Boundaries

This does not establish accurate response for every representable coefficient
combination, extremely ill-conditioned cancellation, singular tangents,
arbitrary softening branches, snap-through, mesh convergence, physical material
validity, large-scale performance or remote recovery. Fused arithmetic reduces
particular rounding/overflow failures; it is not exact real arithmetic.

True overflow still fails rather than being clamped, silently regularized or
converted into convergence. The finite nonconvergence tests retain the committed
state and verify clean replay; they do not assert crash-durable restart.

Evidence is appended to the existing range-reliability profile/claim without
promoting maturity, adding a new operator, or claiming general stability.
Full-workspace, Windows, GUI, remote Agent transport and large-scale performance
suites were not rerun. Benchmark tests above verify correctness classification,
not throughput improvement.
