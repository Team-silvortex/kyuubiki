# PCG Inner-Product Range Reliability

Date: 2026-10-20. Version: daji 3.6.0.

This is bounded PCG inner-product range validation, not general conditioning, arbitrary-precision solving or durable restart.

## Reproduced Limitation

The earlier row-equilibrium repair correctly refused to publish the weak-star
solution prematurely. At `strong=1e300`, `weak=1e-300`, however, it could not
finish the remaining weak equations. Requiring the independently known
displacements `0.4` at the center and `0.2` at the leaves failed with
`iterative solver diverged`.

Diagonal and RHS scaling leave the weak normalized residual near `1e-300`.
Its positive inner products can be near `1e-600`, outside `f64` storage, even
though their ratio and the final physical displacements are representable.
The previous PCG control flow interpreted the rounded zero as breakdown.

## Repair

PCG now retains a mantissa and binary exponent for positive inner products until
forming its alpha and beta ratios. Normal positive dots retain the existing
single accumulation pass. Zero, subnormal, negative or nonfinite direct results
use a bounded two-pass range fallback, with no vector copy or matrix allocation.

The fallback separates operands before multiplying, retains product tails with
fused multiply-add, and uses compensated accumulation. Each pass remains
cooperatively cancellable. Nonfinite operands and genuinely nonpositive results
remain invalid; an unrepresentable ratio does not become an arbitrary finite
step.

This does not weaken the convergence contract. A candidate still passes the
fresh row-equilibrium gate and original-equation validation after restoring the
solution. The raw vector primitives, iteration limit, operator protocol and
engine production code are unchanged.

## Bounded Controls

Seven new inner-product tests cover binary scales from `2^-1000` through
`2^1000`, smallest-subnormal operands, cancellation retaining a product tail,
invalid operands, ordinary division bits, genuinely unrepresentable ratios and
cancellation/replay in both fallback passes.

The formerly failing 1,025-equation weak-star test now checks:

- Jacobi, symmetric Gauss-Seidel and incomplete Cholesky preconditioners.
- `1e300 / 1e-300`, reversed strong/weak scales and `1e250 / 1e-250`.
- One-shot solves, prepared-factor reuse, reversed loads and zero-load replay.
- A 128-iteration ceiling for the manufactured systems.

A new negative control forces a displacement outside the finite range, verifies
failure, then reuses the same prepared solver for a valid solve. A new public
operator test checks independent local force balance through borrowed and owned
APIs. The existing JSON/workflow test now exercises the repaired extreme case,
not just the moderate one, after a genuinely unrepresentable load fails.

There are nine new tests in total; the existing extreme-scale unit and workflow
tests are strengthened, not counted as new tests.

## Reproduction

Run from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_spd::inner_product
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib mixed_scale
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-engine --test spring_mixed_scale_equilibrium --test spring_mixed_scale_workflow
```

This remains finite-precision iteration: extreme cancellation, ill-conditioning,
loss during other scaling operations or an unrepresentable physical solution
can still fail. The targeted systems do not establish a general 600-order
conditioning guarantee or a performance qualification.

## Completed Checks

The macOS AArch64 run used Rust 1.88.0, offline locked dependencies, debug test
builds and existing target caches. No new build root, application packaging or
installation was created.

| Completed group | macOS AArch64 |
| --- | --- |
| Solver unit filter `linear_` | 131 passed, 3 ignored |
| Operator integration targets | 204 passed |
| Engine workflows and headless bridge targets | 28 passed |
| Total, excluding repeated focused runs | 363 passed |

The nine new tests are included in the totals. The integration target list is
the same as the [product-underflow validation run](sparse-product-underflow-reliability-20261020.md#validation),
with one additional public extreme-scale spring test and expanded workflow
assertions. The 10,000-node transient-spring regression remains passing.

Linux source synchronization completed, but both test commands failed to
establish SSH connections. A bounded connection retry also timed out and the
configured address service subsequently failed host-name resolution. Therefore
Linux validation of this change is **not completed**. Earlier Linux results
belong to earlier code and are not counted here.

Follow-up on 2026-10-20: connectivity recovered during the
[vector-range repair](pcg-vector-range-reliability-20261020.md#validation).
The combined tree, including this inner-product change, passed Linux tests:
139 shared linear unit tests and 232 operator/workflow/CLI integration tests.
Those follow-up results are not retroactively counted in the earlier run above.

Local Clippy passed with `-D warnings` for the solver/engine libraries and the
mixed-scale operator/workflow integration targets. Workspace formatting passed.
Native metadata validation passed with 63 profiles and `executed=false`; it is
not a substitute for executing numerical tests.

Tensor structure and commands passed: 13 modules, 11 paradigms, 0 structural
gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall readiness
remains blocked. The three ignored tests are explicit performance benchmarks;
no full-project test, throughput qualification or maturity promotion is claimed.
