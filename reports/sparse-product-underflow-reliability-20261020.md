# Sparse product underflow reliability

Date: 2026-10-20. Version: daji 3.6.0.

This is bounded product-underflow validation, not arbitrary-precision solving, general physical qualification or durable restart.

## Reproduced Defects

The previous repairs protected overflowing products and mixed-scale equations,
but finite direct products could still erase a signal before residual validation.
Three new tests failed before this change:

- `A=[1e-200]`, `b=[0]`, `x=[1e-200]` was accepted as zero residual, although the
  exact residual is nonzero and below the representable `f64` range.
- With `q` the smallest positive `f64`, `A=[q]` and `b=[q]`, the wrong
  displacement `0.75` was accepted. Both `q*0.75` and `q*1.25` round to `q`;
  the exact solution is `1`.
- The compressed PCG candidate gate accepted the same kinds of wrong response.

These are manufactured validation counterexamples, not reports of corrupted
user research outputs. The defect lies in the original-equation acceptance
boundary shared by multiple solver consumers.

## Repair

Nonzero operands whose direct product is zero or subnormal now trigger the
existing mantissa/exponent range path. Finite inputs no longer imply that
intermediate products retained their signal.

The range evaluator now accepts a cloneable entry iterator. Ordinary sparse rows
and compressed CSR rows use the same implementation without copying a CSR row or
building a second matrix. Binary product tails and compensated accumulation are
retained, as are bounded cancellation checkpoints in both range passes.

The normal arithmetic path remains for ordinary products. No global residual
floor, relaxed physical tolerance, engine policy or operator-specific exception
was added. The earlier local subnormal-displacement representation allowance
remains unchanged.

This is not a blanket rejection of subnormal numbers:

- Exact signed subnormal equations still pass.
- Products below the representable range can cancel exactly and pass.
- When individually erased products sum to a representable residual, its sign
  and magnitude are recovered. A 4,096-entry binary-exact control recovers `q`
  rather than zero.
- A genuinely nonzero residual that cannot be represented returns a range error.
  This remains a conservative capability limit, not arbitrary-precision solving.

## Validation

Eight new tests: seven shared-solver unit tests and one public spring operator
test. They cover the three original failures, exact positive controls, signed
accumulation, wide-row cancellation/replay, and borrowed/owned operator APIs with
exactly representable subnormal forces.

Focused commands from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_algebra::residual::underflow_tests
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test spring_mixed_scale_equilibrium --test transient_spring_reliability
```

Both platforms used Rust 1.88.0 with offline locked dependencies, debug test
builds and existing target caches. No new build root, application package or
installation was created.

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 123 passed, 3 ignored | 123 passed, 3 ignored |
| Operator integration targets | 203 passed | 203 passed |
| Engine workflows and headless bridge targets | 28 passed | 28 passed |
| Total, excluding repeated focused runs | 354 passed | 354 passed |

The eight new tests are included in the totals. The integration target list is
the same as the [row-equilibrium validation run](sparse-row-equilibrium-reliability-20261020.md#validation),
with one additional test in `spring_mixed_scale_equilibrium`. This includes
mechanical, thermal, acoustic, advection-diffusion and magnetostatic consumers,
as well as the 10,000-node transient-spring chain.

The three ignored tests are explicit release-mode performance microbenchmarks.
No full-project suite or throughput qualification was run.

Local Clippy passed with `-D warnings` for the solver/engine libraries and the
mixed-scale operator/workflow integration targets. Workspace formatting passed.
Native validation passed with 62 profiles and `executed=false`, which checks
metadata rather than executing numerical tests.

Tensor structure and commands passed: 13 modules, 11 paradigms, 0 structural
gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall readiness
remains blocked; these bounded tests do not promote the system's maturity.
