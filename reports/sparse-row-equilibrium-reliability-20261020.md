# Sparse row equilibrium reliability

Date: 2026-10-20. Version: daji 3.6.0.

Follow-up: [product-underflow validation](sparse-product-underflow-reliability-20261020.md)
extends the shared range path to finite products that lose their signal.

This is bounded mixed-scale equilibrium validation, not general conditioning, industrial physics qualification or durable restart.

## Reproduced defect

The previous range repair handled overflow but retained a finite global
roundoff floor. That floor could still certify a wrong weak equation by borrowing
the scale of an unrelated strong equation. PCG also stopped using a global
normalized residual before the weak subsystem reached equilibrium.

Three red tests independently demonstrated the problem:

- Direct sparse validation accepted a weak-star displacement `[0.25, 0, 0, 0]`
  beside a correctly solved strong equation.
- The 1,025-equation iterative solver returned the same wrong weak solution.
- The real `solve.spring_1d` operator returned weak-center displacement `0.4`
  with five free nodes but `0.25` with 1,025 free nodes.

The weak subsystem has the stiffness matrix

```text
weak * [[ 4, -1, -1, -1],
        [-1,  2,  0,  0],
        [-1,  0,  2,  0],
        [-1,  0,  0,  2]]
```

and load `weak * [1, 0, 0, 0]`. Independent equilibrium gives center
displacement `0.4` and leaf displacement `0.2`. Strong anchored springs have
displacement `1`. The reproducer uses `strong=1e20` and `weak=1e-20`; no
nonfinite input or arithmetic overflow is needed.

## Fix

- Original-equation validation uses each row's own `|b_i| + sum |A_ij*x_j|`,
  with no global force scale borrowed from other rows.
- A small PCG global norm is a candidate, not success. A fresh CSR row-equilibrium
  check must pass before publishing a solution. Failed candidates continue from
  the recomputed residual with a restarted direction.
- Existing iteration limits remain unchanged. The small manufactured mixed-scale
  systems additionally have a 128-iteration test ceiling across Jacobi, symmetric
  Gauss-Seidel and incomplete Cholesky preconditioners.
- The CSR gate has no matrix copy or row-sized allocation. Its row work and
  wide-row interior remain cancellable; interrupted candidates cannot trigger a
  regularized retry.
- The engine and operator protocol are unchanged. A real workflow verifies error
  propagation, clean replay, graph/direct agreement and JSON readback.

## Representation Boundary

A strict relative-only implementation initially regressed the existing large
transient-spring chain: its decaying tail reaches subnormal displacement values.
At that point a representable displacement cannot exactly satisfy every row,
and subsequent Newmark steps also carry tiny nonzero right-hand-side tails.

The final rule uses a local displacement-quantization envelope:

```text
q = smallest positive f64
u_i = sum_j |A_ij| * q/2, only for |x_j| < f64::MIN_POSITIVE
resolved residual = max(|b_i - (A*x)_i| - u_i, 0)
```

The implementation multiplies the coefficient by `q` before halving, so `q/2`
is not rounded to zero first. The envelope applies to loaded and unloaded rows
consistently. It has no dependence on other equations or on a global load norm.
This explicitly admits subnormal-displacement quantization, not arbitrary
absolute force error. A smallest-subnormal stiffness and load with a resolvable
unit displacement still reject the wrong zero solution.

Raw physical residual vectors/norms are **not** rewritten to zero. The range
fallback uses the same rule. Separate controls reject resolvable nonzero loads,
invalid inputs and RHS normalization that erases a nonzero value.

The much wider `strong=1e300`, `weak=1e-300` PCG case still returns an error
because the iteration cannot resolve the remaining work. That is an explicit
capability limit, not a successful zero-response claim.

Subsequent update: [PCG inner-product range repair](pcg-inner-product-range-reliability-20261020.md)
removes that specific limitation for the tested weak-star systems. The paragraph
above records this earlier stage, not the current capability. Genuinely
unrepresentable displacements remain errors.

## Validation

Eleven new tests: eight shared-solver unit tests, two public operator integration
tests and one engine workflow test. Cases include common unit scaling, load
reversal, node renumbering, reversed element orientation, independent force
balance, prepared-factor reuse, range failure and cooperative cancellation.

Focused reproduction commands from the repository root:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_algebra::equilibrium_tests
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test spring_mixed_scale_equilibrium --test transient_spring_reliability
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-engine --test spring_mixed_scale_workflow
```

Both platforms used Rust 1.88.0, offline locked dependencies, debug test builds
and existing target caches. No application packaging or installation was run.

| Completed group | macOS AArch64 | Linux x86_64 |
| --- | --- | --- |
| Solver unit filter `linear_` | 116 passed, 3 ignored | 116 passed, 3 ignored |
| Operator integration targets | 202 passed | 202 passed |
| Engine workflows and headless bridge targets | 28 passed | 28 passed |
| Total, excluding repeated focused runs | 346 passed | 346 passed |

The eleven new tests are included in these totals, not added again. The three
ignored tests are explicit release-mode performance microbenchmarks. No
unfiltered full-suite run or throughput qualification is claimed.

Completed unit command:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib linear_ -- --quiet
```

The operator integration run repeats all 156 cases from the
[previous range validation target list](sparse-residual-range-reliability-20261020.md#validation),
including the five transient-spring reliability tests, and adds these 46 cases:

```text
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver \
  --test spring_mixed_scale_equilibrium --test accuracy_baselines \
  --test spring_vector_closed_form --test spring_vector_refinement \
  --test spring_input_reliability --test truss_2d_review --test truss_3d_review \
  --test thermal_truss_2d_review --test thermal_truss_3d_review \
  --test heat_plane_triangle_2d_review --test heat_plane_quad_2d_review
```

The dispatch run repeats all 27 cases from that report and adds the new
`spring_mixed_scale_workflow` target. The workflow checks JSON dispatch and
clean replay after failure, not process-crash recovery or durable restart.

Local Clippy passed with `-D warnings` for both solver/engine libraries and the
two new integration targets. Workspace formatting passed. Native metadata
validation passed with 61 profiles and `executed=false`; this checks references
and command structure, not numerical execution.

Tensor structure and commands passed: 13 modules, 11 paradigms, 0 structural
gaps, 2 maturity gaps, 23 evidence-grade gaps and 18 P0 gaps. Overall readiness
remains blocked. No full-project coverage or maturity promotion is claimed by
these bounded tests.
