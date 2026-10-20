# PCG Vector-Range Reliability

Date: 2026-10-20. Version: daji 3.6.0.

This is bounded PCG vector-range validation, not general conditioning, arbitrary-precision solving or durable restart.

## Reproduced Boundaries

Five new controls failed before the repair:

- The maximum RHS reduction ignored NaN, allowing the internal PCG entry point
  to return a zero-load success for `[NaN, 0]`.
- Vector norm reduction also excluded NaN rather than reporting an error.
- RHS normalization did not reject invalid scales, nonfinite outputs or an
  erased nonzero load.
- Solution rescaling could return nonfinite outputs without its own error.
- A finite solved diagonal system reported zero residual although fresh
  subtraction retained approximately `1.450417759929779e-216`. Squaring that
  residual erased it.

These are internal admission and diagnostic defects. They do not demonstrate
that the public operator's independent original-equation validation accepted
an incorrect physical solution. That validation remains mandatory.

## Repair

RHS reduction now validates every component before taking the zero-load
shortcut. RHS normalization requires a finite positive scale and refuses
nonfinite results or nonzero loads rounded to zero. Solution rescaling also
requires a finite positive scale and finite outputs.

Recursive updates and fresh residual recomputation share one norm conversion:
positive normal squared sums use the original square-root fast path; zero,
subnormal or nonfinite sums trigger the stable scaled norm. A finite norm can
therefore survive squared overflow, and a small nonzero norm is not erased
merely because its square underflows.

The stable norm rejects nonfinite components and unrepresentable norms. Its
existing accumulation order and 1,024-element cancellation boundaries remain.
The normal squared-sum path does not scan the vector a second time; exceptional
paths borrow the existing residual, without allocating another vector.

The unreachable initial nonpositive-dot zero-solution shortcut was removed.
For an admitted nonzero RHS normalized by its finite maximum, at least one
component has magnitude one, so the initial norm cannot satisfy the relative
convergence tolerance.

Finite subnormal solution rounding remains allowed at rescaling. Whether that
rounding is physically acceptable is decided by the existing row-local
equilibrium checks and final original-equation validation, not an unconditional
ban on small displacements. No operator protocol or engine production code
changed.

## Controls

Eight new tests cover invalid values across vector block boundaries, invalid
scales, erased nonzero loads, exact subnormal normalization, nonfinite solution
outputs, finite quantization, failure followed by a clean solve, nonzero tiny
PCG profile residuals, binary norm references, squared overflow, cancellation
and the no-second-scan normal path.

The norm references include the smallest subnormal and binary scales through
`2^1000`. Fresh and recursive residual paths use the same conversion. The
existing finite-vector bitwise regression is retained; its former requirement
to preserve NaN-ignoring behavior is replaced with explicit rejection tests.

Run from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_spd::vectors
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver -p kyuubiki-engine --test spring_mixed_scale_equilibrium --test transient_spring_reliability --test spring_mixed_scale_workflow
```

## Validation

Both platforms use Rust 1.88.0, offline locked dependencies, debug tests and
existing target caches. No new build root, application package or installation
was created.

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 139 passed, 3 ignored | 139 passed, 3 ignored |
| Operator integration targets | 204 passed | 204 passed |
| Engine workflow and CLI targets | 28 passed | 28 passed |
| Total, excluding repeated focused runs | 371 passed | 371 passed |

The integration selection matches the
[inner-product range run](pcg-inner-product-range-reliability-20261020.md#completed-checks).
The three ignored tests are explicit performance benchmarks, not numerical
tests silently excluded from the pass count. This is not a full-project test
run or a throughput qualification.

Linux SSH connectivity recovered after the address-discovery hostname failed
to resolve. Source was incrementally synchronized into the existing test
snapshot without deleting files or changing service configuration. These runs
also include the preceding inner-product repair, whose Linux checks had been
blocked by connection failures.

Local Clippy passed with `-D warnings` for the solver/engine libraries and
mixed-scale operator/workflow integration targets. Native validation of 64
operator profiles passed with `executed=false`; it validates metadata, not
numerical execution.

Workspace formatting and whitespace checks passed. Tensor structure and command
validation passed: 13 modules, 11 paradigms, 0 structural gaps, 2 maturity gaps,
23 evidence-grade gaps and 18 P0 gaps. The prior vector-recovery evidence now
references the renamed finite-order regression instead of preserving invalid
NaN behavior. No maturity promotion was made; overall readiness remains blocked.
