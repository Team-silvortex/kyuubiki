# Sparse-Scaling Range Reliability

Date: 2026-10-20. Version: daji 3.6.0.

This is bounded sparse-scaling range validation, not general conditioning, physical material qualification or durable restart.

## Reproduced Failures

Diagonal congruence scaling evaluated `a_ij * d_i * d_j` from left to right.
For a coefficient `2^-1000` and factors `2^-500`, `2^500`, the first
multiplication rounded to zero even though the final coefficient remains
exactly `2^-1000`. Reversing row and column preserved the coefficient, so
one half of a symmetric coupling was erased.

The same arithmetic existed in both direct compressed setup and the sparse
matrix materialized for regularized retries. Additional controls cover a
partially rounded subnormal intermediate and an overflowing intermediate
whose final product is representable.

A manufactured positive-definite star with 1,025 equations has a strong
diagonal `2^1000`, three weak diagonals and couplings `2^-1000`, a center load
of `3` and leaf loads of `1`. Its exact center displacement is zero and its
leaf displacements are `2^1000`. The prior solve failed despite the finite
reference solution.

Fixing coefficient scaling exposed a second boundary: a temporary PCG
candidate could have products spanning too wide a range for reliable residual
normalization. The candidate gate aborted the solve instead of rejecting that
candidate and allowing further iterations. The final reference solution does
not have this unresolved residual.

## Repair

Normal intermediate and final products retain their original arithmetic order.
Exceptional products split finite operands into binary mantissas and exponents,
combine exponents, then restore the final coefficient. Representable subnormal
coefficients remain usable; a nonzero coefficient that would become zero, or
a genuinely overflowing coefficient, returns an error.

Compressed setup, retry matrix construction and scaled diagonal magnitude use
the same coefficient routine. Scaling factors are checked for matching
dimensions and finite positive values before matrix traversal.

Residual range failures now have an internal classification. The iterative
candidate gate converts only unresolved numeric range into candidate rejection.
Invalid operands and cancellation still propagate as errors. Final
original-equation validation continues to reject unresolved range, so this is
not a weaker convergence certificate or a new permission to publish uncertain
answers.

The binary split/restore helpers previously duplicated by residual validation
and inner products are shared inside the solver crate. No operator protocol,
engine production code, runtime dependency or shell/Node tooling was added.

## Controls

Six new unit tests cover:

- Signed exact couplings surviving zero, rounded-subnormal and overflowing
  intermediate products in sparse and compressed layouts.
- The 1,025-equation star under Jacobi, symmetric Gauss-Seidel and incomplete
  Cholesky, with node renumbering, prepared-factor reuse and reversed loads.
- Candidate rejection without relaxing final validation or masking a later
  NaN component.
- Invalid scaling dimensions/factors and truly unrepresentable coefficients.
- Exactly representable smallest-subnormal couplings.
- Cancellation inside a 2,049-entry scaling row and clean replay.

One new public spring-operator test uses a stiff anchor and three weak branches
with balancing center/leaf loads. Borrowed and owned entry points, both load
signs, reversed node numbering and element orientation are checked. Expected
displacements, assembled force balance and total strain energy are independently
verified. This is a numerical manufactured model, not a claim that enormous
linear-spring deformations represent a real material.

The existing ordinary-arithmetic bitwise regression remains passing. The
scalar overflow control is an arithmetic test, not an assertion that its
arbitrary matrix is positive definite.

## Reproduction

Run from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_algebra::scaling_range_tests
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-engine --test spring_mixed_scale_equilibrium --test transient_spring_reliability --test spring_mixed_scale_workflow
```

## Validation

Rust 1.88.0, offline locked dependencies and existing debug target caches were
used. No new build root, application package or installation was created.

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 145 passed, 3 ignored | 145 passed, 3 ignored |
| Operator integration targets | 205 passed | 205 passed |
| Engine workflow and CLI targets | 28 passed | 28 passed |
| Total, excluding repeated focused runs | 378 passed | 378 passed |

The target selection matches the
[vector-range run](pcg-vector-range-reliability-20261020.md#validation), with one
additional public spring test. Three ignored tests are explicit performance
benchmarks. This is not a full-project test run or a throughput qualification.

Local Clippy passed with `-D warnings` for solver/engine libraries and the
mixed-scale operator/workflow integration targets. Workspace formatting and
whitespace checks passed; changed source/config files stay below 800 lines.

Native operator validation passed with 65 profiles and `executed=false`.
Tensor structure and commands passed: 13 modules, 11 paradigms, 0 structural
gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Metadata validation
is not numerical execution, and overall readiness remains blocked. No maturity
promotion or general extreme-conditioning guarantee is claimed.
