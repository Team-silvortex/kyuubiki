# Sparse Modal Product Range Reliability

## Scope And Reproductions

On 2026-10-02, the daji 3.4.3 working tree based on `99e494d1` was checked
with the preceding uncommitted modal changes preserved. This round fixes
avoidable intermediate range loss in the physical sparse product
`M^-1/2 K M^-1/2 x`. Changes stay inside Solver; Agent, Engine, SDK and TaskIR
responsibilities, numerical acceptance tolerances and search budgets do not
change.

Eight new regressions reproduced seven failures before production changes.
One common-scale discrete reference already passed. The heterogeneous-mass
fixture has physical diagonal stiffness and mass `[1e200, 1e-200]` and
symmetric stiffness couplings `-1e-300` or `-1e-220`. One product direction
previously lost or rounded the weak term before row mass weighting could
recover it. Both final couplings are representable. Unit-basis probes and
reversed DOF order now retain them in plain and compensated products.

Other fixtures separately cover input underflow, input overflow and
stiffness-product overflow with representable final results. For example,
stiffness `1e308`, mass `1e200` and input `1e-250` have a final result near
`1e-142`; the old input scaling returned zero. Stiffness `1e-300`, mass
`1e-300` and input `1e200` instead overflowed input scaling despite a final
result near `1e200`. Plain products also accepted non-finite inputs/outputs.
The first regression run exposed a missing input-preparation cancellation
phase and rounding error in a long plain row sum at extreme common scale.

A follow-up minimum-subnormal probe has stiffness two, mass one half and
input three times the minimum subnormal. The retained legacy algorithm
returns eleven subnormal units instead of the independently known twelve.
The final exceptional path avoids that compounded intermediate rounding.

## Product Routing And Storage

`modal_sparse_product.rs` owns vector validation and both product entrypoints.
The allocating constructor caches one boolean indicating whether the
immutable stiffness and inverse-mass factors fit the staged safe interval.
Its scans have cancellation checkpoints. Per-call vector checks determine
whether the original CSR product can be used without risky intermediate
range. Nonzero factors in `[1e-60, 1e60]` keep four-factor products and a
usize-bounded row sum inside normal finite f64 range. Zero vector entries
are accepted. This interval selects an algorithm, not model admissibility,
physical accuracy or coupling pruning.

Ordinary products keep the original multiplication and summation order.
Their output buffer is explicitly mass-weighted in place. Vector preparation
and output checks run in 64-entry blocks, avoiding a checkpoint function call
per entry without changing checkpoint granularity.
The ordinary compensated path keeps its mass-scaling FMA correction and
compensated physical row sums.

`modal_sparse_product_range.rs` handles exceptional scales by separating the
four original physical factors into bounded mantissas and binary exponents.
Only the combined exponent is restored after multiplication. Compensated
calls retain multiplication roundoff with FMA and compensate row sums. The
plain route remains an uncompensated sum. No normalized coefficient cache,
dense fallback, dependency or global mutable state is added. The operator
stores only its existing CSR/mass data plus the routing boolean; both routes
remain `O(nnz + n)` work with `O(n)` output/scratch storage.

This deliberately does not use rounded normalized coefficients as the
physical residual oracle. Non-finite vectors, contributions or row results
return errors. Both input and row phases are cancelable, including 1024-entry
wide-row boundaries. Canceled calls return no successful vector and do not
mutate the operator, so the same instance can be replayed.

## Regression And Public Recovery

The final addition is fifteen cases: thirteen kernel tests, including a non-gating
local comparison, and two public solver recovery tests.

- Weak couplings retain their sign and magnitude in both directions and
  after DOF reordering; references are explicit values, not a product of the
  new normalized matrix.
- Separate signed inputs exercise all three avoidable underflow/overflow
  stages. Intrinsically non-finite results still fail, followed by finite
  replay on the same operator.
- Independent two-DOF products agree at common scales `1e-318`, `1e-200`,
  `1` and `1e200`.
- Binary common scales `2^-600` and `2^600` retain the FMA residual
  `-(2^-27)^2` that ordinary rounded multiplication loses.
- Input, row and wide-row cancellation stop inside the product call rather
  than relying on observer-scope exit to turn a successful return into failure.
- Constructor mass/stiffness routing scans and ordinary output weighting
  cancel inside their calls, with successful replay and no cached vector state.
- An ordinary matrix accepts tiny, huge and minimum-subnormal signed inputs
  through its exceptional route and still produces legacy bits on later
  ordinary calls. The minimum-subnormal probe retains the twelve-unit result.
- Public 129-segment planar/spatial axial chains at `2^-600` and `2^600`
  cancel in the first sparse product, in both preparation and row phases.
  Replay retains the discrete closed-form frequency, unit physical shape and
  restrained zeros. An independently assembled physical residual cancels
  the common scale analytically and stays below `1e-9`.
- Ordinary products match the test-only legacy implementation bit for bit
  for sampled heterogeneous masses and mixed-sign/zero inputs.

## Bounded Debug Comparisons

The isolated macOS ARM64 Rust 1.88.0 debug comparison alternates the legacy
and checked paths over five samples, with sixteen calls per sample. Timings
include allocation and product work, not matrix assembly, operator preparation
or eigensolve. No assertion depends on timings or speed ratios.

| Free DOFs | Stored entries | Legacy median us | Checked median us |
| ---: | ---: | ---: | ---: |
| 1024 | 3070 | 89 | 99 |
| 8192 | 24574 | 722 | 798 |

An earlier per-entry checkpoint snapshot measured legacy/checked medians
of 173/210 and 688/896 us. The first block-level snapshot measured 174/179
and 685/768 us. Host load and clock behavior affect the absolute timings;
they are not controlled release-performance comparisons. Finite input/output
validation and routing still have a measured cost. These samples
do not establish zero overhead or an overall solver speedup. The exceptional
route performs more per-entry work and is not a large extreme-scale throughput
qualification.

These are unoptimized local measurements, not release-build throughput.

## Validation Results

| Lane | Result |
| --- | --- |
| Final sparse product target | 19 passed, 0 failed, 0 ignored |
| Public axial chain target | 16 passed, 0 failed, 0 ignored |
| Full Solver suite including integration and doc harnesses | 1300 passed, 0 failed, 9 existing ignored; 186 result groups |
| Engine library | 637 passed, 0 failed, 1 existing ignored |
| Full CLI suite including live modal Agent and Rust headless journeys | 488 passed, 0 failed, 4 existing ignored; 43 result groups |
| Executed modal profile | 28 commands, 266 passed, 0 failed, 0 ignored |
| Strict Solver Clippy on all targets | Passed with warnings denied |

The overlapping lanes must not be summed as distinct coverage. The executed
profile artifact has `executed=true` and `ok=true`, and its retained-report
validation passed. The profile includes the existing physical bending,
complete-spectrum, TaskIR capability, live TCP Agent, cancellation and
headless recovery checks. No ignored test, dependency or tolerance exception
was added. This is not an installed application or remote Linux verification.

Formatting, whitespace, documentation book/inventory and source-800 /
document-2000 organization checks passed. The tensor structure and command
checks pass with 13 modules, 11 paradigms and zero structural gaps. Release
readiness stays blocked with four maturity gaps, sixteen evidence-grade gaps
and eleven P0 gaps. The new claims retain their bounded numerical/recovery
and local measurement scope instead of promoting general qualification.

Final SHA-256 fingerprints, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_sparse.rs` | `a539e77fc253e911d8fba45b6d6fbd4e7ba7469bbeeb571041e94f8237b36129` |
| `solver/src/modal_sparse_product.rs` | `fa303b02c33819b7ebef994e2fdee77e635f5f566a60a88b4f9a41f30e2296b6` |
| `solver/src/modal_sparse_product_range.rs` | `73d5f73ed8c369ccf3a8f66869e29fdd7cd4ce6cf0345261046ec92270de4d22` |
| `solver/src/modal_sparse_product_tests.rs` | `93fd867e17ac9361afdc6c83ab44417a16c8c75a048d2e097e3691ef6752fabd` |
| `solver/src/modal_sparse_range_tests.rs` | `c3d6f0547c64a38a3ffbeee63708e1bffa2b83533fcd1719f4ef6b827bdbbef2` |
| `solver/tests/modal_chain_reliability.rs` | `f93cd3f6fdf94db1069f7dad983d602c9ec9994e929a053a02be4f3c458e62ee` |
| `solver/tests/modal_chain_reliability/range.rs` | `d21a2001f4bca4bdfe5fffde8fc0b1bc9352994209b03a75e318913abc8e4648` |

## Remaining Boundaries

This is bounded sparse modal product validation, not general modal qualification.
It does not provide arbitrary-precision summation, rescue rows whose individual
terms intrinsically exceed f64 range, or promise preservation of contributions
intrinsically below the minimum subnormal. Ordinary cancellation roundoff is
not changed into a compensated calculation. Rounded tridiagonal extraction
and its preprocessing cancellation coverage remain a separate follow-up.

The existing unresolved 128-element bending spectrum still fails within its
original budget; no convergence gate is relaxed. Broader material/industrial
correlation, complete spatial spectra, sustained large-scale benchmarks and
installed/remote Agent qualification remain separate obligations.
