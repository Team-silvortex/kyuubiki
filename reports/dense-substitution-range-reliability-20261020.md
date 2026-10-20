# Dense Substitution Range Reliability

Date: 2026-10-20

Version: daji 3.6.0

## Reproduced failures

Row equilibration alone does not protect substitution when an ordinary
matrix is paired with a very large RHS. Three new tests failed before
this change:

- With `h = 2^1023`, `A = [[2, -1], [-1, 2]]` and `b = [h, h]`,
  the exact solution is `[h, h]`. The old back substitution formed
  `h + h` before dividing by `2`, reporting divergence.
- The upper-triangular matrix `[[1, 2, -2], [0, 1, 0], [0, 0, 1]]`
  with `b = [1, h, h]` has solution `[1, h, h]`. Separately overflowing
  products destroyed their cancellation during back substitution.
- The lower-triangular matrix `[[1, 0, 0], [0, 1, 0], [2, -2, 4]]`
  with `b = [h, h, 4]` has solution `[h, h, 1]`. Forward substitution
  suffered the same product-cancellation failure.

These are manufactured arithmetic controls, not physical material models.

## Repair and bounds

The common row operation computes `(rhs - dot) / pivot` on the previous
arithmetic path for ordinary data. It detects non-normal nonzero products,
subnormal or non-finite accumulation/numerators, and unrepresentable results.

Only affected rows take two additional passes, retaining binary exponents.
Product mantissas and fused multiply-add tails feed a compensated sum.
The pivot exponent is combined before restoring the final solution,
so a finite quotient does not require a representable unscaled numerator.
The fallback uses constant extra row storage; it does not allocate a row copy.

Non-finite inputs, zero pivots, mismatched dimensions, lossy normalization,
and nonzero solutions rounded entirely to zero remain explicit errors.
The range normalizer is deliberately bounded: it can still reject an
ultimately representable solution when the row's intermediate dynamic range
cannot be retained without loss. It is not arbitrary-precision arithmetic.

Existing singular-pivot and original-equation residual tolerances are unchanged.
The new `dense_substitution_range` observation stage is appended without
renumbering existing stages. Cancellation and failed RHS solves leave the
prepared factor immutable. No operator/engine protocol or runtime dependency
was added.

## Controls

Nine new tests cover:

- The three exact failures above.
- An overflowing product pair whose nonzero difference exists only in the
  fused product tail: `2(1+d) * 2^1023(1-d) - 2 * 2^1023 = -2^970`,
  with `d = 2^-27`.
- Erased and partially rounded subnormal products under both load signs.
- Fully connected SPD direct/prepared parity with an exact finite solution
  and zero original-equation residual.
- Lossy normalization, true overflow/underflow, invalid late operands,
  wrong dimensions and zero pivots.
- Ordinary row bit patterns, signed zero, and absence of fallback passes.
- Cancellation in both passes of a 2,049-entry row and clean factor replay
  after interruption or a failed RHS solve.

## Validation

Rust 1.88.0, offline locked dependencies and existing target caches:

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 161 passed, 3 ignored | 161 passed, 3 ignored |
| Solver unit filter `solver_control` | 12 passed | 12 passed |
| Operator integration targets | 205 passed | 205 passed |
| Engine workflow and CLI targets | 28 passed | 28 passed |
| Total, excluding repeated focused runs | 406 passed | 406 passed |

The nine new substitution tests are included in `linear_`, not counted twice.
The integration selection matches the
[dense-equation run](dense-equation-range-reliability-20261020.md#execution).
Three ignored tests are existing opt-in performance benchmarks. This is not
a full-project test run or a throughput qualification.

Local Clippy passed with `-D warnings` for solver/engine libraries and the
mixed-scale operator/workflow integration targets. Workspace formatting and
whitespace checks passed; changed source/config files remain below 800 lines.
No new build root, application package or installation was created.

Native operator validation passed with 67 profiles and `executed=false`.
Tensor structure and command checks passed: 13 modules, 11 paradigms,
0 structural gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps.
Overall readiness remains blocked. The new evidence is component-specific;
metadata checks do not execute numerical tests or imply general qualification.

Focused reproduction from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_dense::substitution_tests -- --nocapture
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_ -- --quiet
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib solver_control -- --quiet
```

This is bounded dense-substitution range validation, not general conditioning, arbitrary-precision solving, physical qualification or durable restart.
