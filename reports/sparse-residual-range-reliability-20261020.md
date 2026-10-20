# Sparse residual range reliability

Date: 2026-10-20. Version: daji 3.6.0.

Follow-up: [row-local equilibrium validation](sparse-row-equilibrium-reliability-20261020.md)
subsequently removes the finite global error floor as well. Statements below
about retaining the normal-range policy describe this earlier range-only stage.

## Scope

This is bounded local sparse residual validation, not arbitrary-precision solving, general physical qualification or durable restart.

The shared linear solver checks the original equations after a solve. Earlier
tridiagonal range repairs did not fix this separate, general sparse validation
layer. This change remains in the solver crate; the engine, operator protocol
and agent execution policy are unchanged.

## Reproduced defects

Four minimal regression tests failed before the sparse residual repair:

1. With `s = 2^1023`, `A = [s]`, `b = [1.5s]`, and `x = [1]`, the finite
   wrong solution had relative error `0.2`, but an overflowing denominator
   produced zero.
2. In `A = diag(s, 1)`, `b = [s, 1]`, `x = [1, 2]`, an infinite global
   scale hid the second equation's relative error `1/3`.
3. For `A = s * [[1, -0.5], [-0.5, 1]]`, `b = [1.5s, 1.5s]`, the exact
   solution `[3, 3]` produced infinite intermediate products. The old residual
   vector was nonfinite and the prepared solver rejected its correct result.
4. Mismatched vector lengths were silently truncated by `zip`, so not all
   equations were checked.

A subsequent ten-test run had nine passes and one failure: a nonfinite solution
component in an empty equation escaped the matrix products and was accepted.
The final admission now explicitly scans the complete solution with cooperative
cancellation checkpoints.

## Implementation

- Keep the normal-range accumulation order and existing roundoff-floor policy.
- Enter the range-safe path if a residual, equation scale or derived floor is
  nonfinite. Relative validation then recomputes all rows with local scales,
  rather than borrowing an invalid global floor.
- Split finite operands into binary mantissas and exponents before multiplication.
  Preserve the product tail with fused multiply-add and use compensated sums for
  cancellation. Restore signed physical residuals, not only ratios.
- Reject normalization that loses a nonzero value or partially rounds a
  subnormal component. A genuinely unrepresentable residual cannot be accepted
  as zero.
- Validate exact dimensions and finite positive tolerances. Invalid stored
  columns return errors rather than indexing panics.
- Check cancellation in both row passes, including a 4,097-entry regression row.
  The fallback uses constant extra storage per row and linear work.

The fallback is intentionally conservative. Very wide exponent spreads may
return an explicit range-loss error even when an arbitrary-precision method
could recover a finite answer. It is not a general cure for ill-conditioning.
The normal path's numerical policy is unchanged; its performance is not newly
qualified by this correctness run.

## Validation

The focused suite contains twelve tests, including manufactured signed residuals
at binary scales `2^-1074`, `2^-1000`, `1`, and `2^1000`, invalid operands and
tolerances, prepared-solver reuse, false-acceptance controls, and cancellation/replay.
An exact `(1+d)(1-d) = 1-d^2` reference also checks that cancellation preserves
the product's rounding tail rather than publishing zero from cancelling heads.

Run from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_algebra::residual::range_tests
```

Both platforms used Rust 1.88.0, offline locked dependencies, debug test builds
and existing target caches. No application packaging or installation was run.

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 108 passed, 3 ignored | 108 passed, 3 ignored |
| Mechanical and field operator integration targets below | 156 passed | 156 passed |
| Engine workflows and headless bridge targets below | 27 passed | 27 passed |
| Total completed, excluding repeated focused runs | 291 passed | 291 passed |

The twelve new range tests are included in the 108 unit passes, not added again.
The three ignored tests are explicit release-mode performance microbenchmarks.
An initial unfiltered solver unit run on each platform was deliberately
interrupted during unrelated, long-running modal candidate searches. Neither
partial run is counted as a full-suite pass. No full-project test or performance
qualification is claimed.

Completed unit command:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_
```

Completed operator targets:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver \
  --test nonlinear_spring_equation_scaling --test nonlinear_spring_tangent_pivot \
  --test nonlinear_spring_softening_arithmetic --test nonlinear_spring_scaling \
  --test nonlinear_spring_tolerance --test nonlinear_spring_globalization \
  --test nonlinear_spring_topology --test nonlinear_spring_recovery \
  --test nonlinear_spring_range_reliability --test nonlinear_spring_reliability \
  --test nonlinear_spring_1d_closed_form --test nonlinear_spring_1d_review \
  --test contact_gap_reliability --test contact_gap_1d_closed_form \
  --test contact_gap_1d_refinement --test contact_gap_1d_review \
  --test mechanical_convergence --test acoustic_bar_closed_form \
  --test acoustic_bar_refinement --test acoustic_bar_reliability \
  --test advection_diffusion_bar_closed_form --test advection_diffusion_bar_refinement \
  --test advection_diffusion_bar_reliability --test advection_diffusion_bar_upwind \
  --test magnetostatic_bar_closed_form --test magnetostatic_bar_refinement \
  --test magnetostatic_bar_reliability --test thermal_bar_1d_closed_form \
  --test dynamic_spring_closed_form --test transient_spring_reliability \
  --test harmonic_spring_reliability --test spring_1d_closed_form \
  --test spring_1d_refinement
```

Completed dispatch targets:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-engine -p kyuubiki-cli \
  --test nonlinear_spring_recovery_workflow --test nonlinear_spring_topology_workflow \
  --test nonlinear_spring_workflow --test contact_gap_workflow \
  --test nonlinear_spring_range_operator
```

Local library Clippy with `-D warnings`, workspace formatting and `git diff
--check` passed. Native metadata validation passed with 60 profiles and
`executed=false`. Tensor structure and commands passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps, 18 P0 gaps; overall
readiness remains blocked.

Metadata checks validate evidence references and command structure; they are not
substitutes for executing the numerical tests. No maturity promotion is claimed.
