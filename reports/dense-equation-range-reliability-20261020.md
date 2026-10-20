# Dense Equation Range Reliability

Date: 2026-10-20

Version: daji 3.6.0

## Reproduced failures

The dense LU backend still operated directly on extreme-scale equations.
Scaled pivot selection alone did not protect elimination or substitution:

- For `h = 2^1023`, `A = h * [[1, -1/2], [-1/2, 1]]` and
  `b = [h, h]`, the exact solution is `[2, 2]`. The old backend reported
  `linear system back substitution diverged`: a numerator overflowed before
  division by the pivot.
- For `q = f64::from_bits(1)`, `A = q * [[4, -2], [-2, 4]]` and
  `b = [q, q]`, the exact solution is `[1/2, 1/2]`. The old dense backend
  returned `[1/2, 1/3]` after rounding away a forward-substitution term.

Both new regression tests failed before the implementation changed.
The second result describes the raw dense backend; it does not claim that
the higher-level original-equation validator accepted the wrong solution.

## Repair

When a row maximum has a binary exponent outside `[-500, 500]`, normalize
the equations by exact powers of two before LU factorization. The ordinary
path retains its previous factorization and substitution arithmetic.

Keep scaling exponents in original row order and use the pivot permutation
when scaling each new RHS. Every coefficient and RHS must round-trip
exactly through scaling. Non-finite, erased or partially rounded data is an
explicit error rather than an altered equation.

The exceptional path adds one exponent per row, not a second dense matrix.
Scaling uses the shared binary exponent helper. New `dense_row_scale` and
`dense_rhs_scale` control stages permit interruption and preserve immutable
prepared factors. Existing stage discriminants are unchanged.

The prepared and direct SPD wrappers still validate the original equations.
No residual tolerance, singular-pivot threshold, regularization policy or
operator/engine dispatch contract was relaxed.

## Regression scope

- Large and subnormal binary manufactured systems, including signed loads.
- Mixed row scales, six equation permutations, zero loads and repeated solves.
- A fully connected SPD system with direct/prepared parity and zero original
  residual, including minimum-subnormal and near-maximum coefficient scales.
- Erased and partially rounded coefficients/RHS, overflow, NaN, infinity,
  wrong dimensions and a singular scaled matrix.
- Cancellation at scaling entry, 1024 entries, the final entry, RHS scaling
  and substitution, followed by a clean solve using the same factor.
- Ordinary row scales and the empty system retain their existing behavior.

## Execution

Rust 1.88.0, offline locked dependency resolution and existing target caches
were used on both hosts:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 152 passed, 3 ignored | 152 passed, 3 ignored |
| Solver unit filter `solver_control` | 12 passed | 12 passed |
| Operator integration targets | 205 passed | 205 passed |
| Engine workflow and CLI targets | 28 passed | 28 passed |
| Total, excluding repeated focused runs | 397 passed | 397 passed |

The seven new dense tests are included in `linear_`, not counted twice.
The integration selection matches the
[sparse-scaling run](sparse-scaling-range-reliability-20261020.md#validation).
Three ignored tests are existing opt-in performance benchmarks.
This was not a full-project test run or a throughput qualification.

Local Clippy passed with `-D warnings` for solver/engine libraries and the
mixed-scale operator/workflow integration targets. Workspace formatting and
whitespace checks passed. Changed source/config files remain below 800 lines.
No new build root, application package or installation was created.

Focused reproduction command:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_dense::range_tests -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_ -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib solver_control -- --quiet
```

Native operator validation passed with 66 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. The tensor evidence is limited to this
component regression; metadata checks do not execute numerical tests or confer
general industrial qualification.

This is bounded dense-equation range validation, not general conditioning, arbitrary-precision solving, physical qualification or durable restart.
