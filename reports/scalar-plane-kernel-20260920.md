# Scalar Plane Kernel: 2026-09-20

## Scope And Results

This local change covers six existing Rust solvers: heat, electrostatic and
magnetostatic triangle and split-quad planes. It changes no input/output schema
and does not introduce a new physical model. Base revision: `3007110a`.

Two numerical defects were reproduced before the kernel change:

- On skew patches with a constant field of `2^40`, absolute-value gradient
  summation produced a false y-gradient of -0.0001220703125 for triangles and
  -0.00006103515625 for quads. The relative formulation gives zero.
- A checkerboard potential on a unit quad gave zero electrostatic/magnetostatic
  energy although the analytic and explicit-triangle value is 0.625. Energy
  now integrates both subtriangles before averaging; unequal-area energy is
  separately checked against 1.40625.

All eight original regression tests failed before the fix and passed after it.
The final 12 integration tests additionally cover positive/negative shifts,
nonzero sources, heterogeneous coefficients, reversed connectivity and absolute
value restoration. The 34-division meshes contain 1,225 nodes and 1,089 free
DOFs, crossing the dense/sparse solver boundary; these are not scale benchmarks.
Four new kernel unit tests cover nonfinite geometry/stiffness, overflowing
prescribed ranges, very unequal subtriangle energy weights, and cancellation
followed by a clean replay.

The combined regression run passed 281 solver unit tests and 48 integration
tests across 18 targets, with zero failures. Six opt-in unit tests were ignored
by that ordinary run; the new precompute benchmark was executed explicitly.
Existing conduction, field review, orientation, refinement, preparation and
postprocessing cancellation tests remained passing. Full remote/installed
Agent, Windows and million-node runs were not performed in this round.

The optimized release repeat passed 281 units and all 12 new integration tests.
The Engine's `static`-filtered unit suite passed 48 tests, including related
electromagnetic bridges/diagnostics. Both thermal-plane-patch (8 commands) and
electromagnetic-plane-patch (10 commands) profiles were also executed through
the native validation runner, with all commands successful. These overlapping
runs are not added together as a unique-test coverage percentage.

Validation-profile structure, tensor structure and repository organization
checks passed. Strict Clippy encountered the pre-existing `type_complexity`
warning in `linear_ic0.rs`; with only that lint exempted, the solver library and
new integration target passed with all other warnings denied.

## Implementation Boundary

The scalar element geometry, stiffness and gradient routines are shared rather
than copied across three operator families. Electrostatic/magnetostatic quads
no longer construct two temporary string IDs per element. Geometry/stiffness
overflow fails before assembly.

The solve uses `u - reference`, including prescribed reduction, and retains
relative values until field/energy postprocessing is complete. Nodal output and
element averages restore the reference; prescribed output preserves the original
input value exactly. Input `f64` precision is still a limit. The new reference
pass polls cancellation; it does not add a resumable numerical checkpoint.

Quad vector outputs remain area means, while energy is the mean of the squared
subtriangle gradients, not the square of their mean. Consumers must use the
energy fields rather than recompute energy from mean vectors. Older nonuniform
quad energy results need explicit recomputation only if they will be reused.
Disposable development outputs may instead be discarded without migration,
backfill or an old-result compatibility layer; this does not remove solver,
example, regression or data-management capabilities. Stored artifacts are not
silently mutated. No material certification, Q4 upgrade or global tensor promotion is
claimed. Conduction shares the element helper but its terminal/reference solve
has not been redesigned in this change.

The subsequent [conduction reference recovery](electric-conduction-reference-20260920.md)
change covers that separate terminal/contact solve and power-recovery boundary.

## Local Precompute Measurement

Host: Apple M2, macOS ARM64, rustc 1.88.0. Release profile, seven samples of
100,000 calls per family; inputs and outputs are black-boxed. Input setup and
serialization are outside timing. Baseline and candidate use the same benchmark
source, each measured in its own build on this host.

| Family | Before, median ns/element | After, median ns/element |
| --- | ---: | ---: |
| Electrostatic quad | 183.835 | 34.221 |
| Magnetostatic quad | 156.862 | 34.405 |

The preceding candidate run measured 34.552 and 34.081 ns respectively; the
table records the final-code repeat.

This is a precompute microbenchmark, not end-to-end solver throughput, a memory
capacity test or a 1M-node qualification. No global speedup bound is claimed.
Host scheduling and frequency can affect these point estimates.

## Reproduction

From the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --test scalar_plane_reference_invariance
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --lib
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-solver --release --lib scalar_plane_quad_precompute_benchmark -- --ignored --nocapture --test-threads=1
```

The new regression is registered in the thermal-plane-patch and
electromagnetic-plane-patch validation profiles. The tensor records bounded
`verified` numerical/microbenchmark evidence only; existing broader
qualification obligations are unchanged.
