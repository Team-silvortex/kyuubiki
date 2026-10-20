# Linux Modal Numerical Portability, 2026-10-18

## Scope and Diagnosis

- Baseline: `dcd0490fdd46c06f2648e4d708d8d1e732068b52` (daji 3.6.6).
- Compared macOS AArch64 and Linux x86_64 with the same Rust 1.88.0 toolchain.
- This follows the Linux numerical failures in [the CI repair report](ci-repair-20261018.md).
- No desktop rebuild, commit, push or hosted CI rerun is part of this repair.

The 128-segment layered bending fixture was traced at unit, large (`1e14`) and
tiny (`1e-10`) coordinate scales. Mass, physical stiffness and normalized
stiffness were bit-identical on both machines. The first divergence was the
Jacobi eigenpair, before residual refinement or the candidate handoff.

The rotation radius used the host's `f64::hypot`. Its final rounding differs
between the two system math libraries. A one-bit radius change changes later
rotations and the rounded seed in these ill-conditioned fixtures. Subsequent
refinement can consequently enter a different rounding-limited recovery path.
Changing the Rust toolchain did not resolve this upstream difference.

Observed failures before this repair included:

- Coherent-phase translated-polisher residual `2.1605263156202754e-8`, outside
  its original observation band, though still correctly rejected by the `1e-8`
  production gate.
- Banded handoff exhausted two policies and eight certificates before reaching
  the physical fault-injection and cancellation checks.
- CLI planar reassembly failed the physical `1e-8` gate with residual
  `1.0483577376274835e-8`; spatial repeated-frequency solving also failed in CI.

## Repair and Invariants

`workers/rust/crates/solver/src/modal_norm.rs` defines a fixed arithmetic path:
absolute-value ordering, exact power-of-two scaling, one rounded larger square,
one fused multiply-add for the smaller square, square root, and rescaling.
Special handling covers zeros, non-finite values, tiny secondary components,
subnormals and overflow. This is a reproducible modal primitive, not a claim
to implement a universally correctly-rounded system math library.

`modal_math.rs` uses it for Jacobi rotation radii and chunked output-shape norms.
The change does not add a dependency, platform-specific branch, retry policy,
candidate algorithm, fallback or production path from a research-only module.

Unchanged:

- Production normalized and physical residual gate: `1e-8`.
- Jacobi convergence tolerance, sweep limit and refinement budget.
- Existing observation bands, reference roots and exact-replay assertions.
- Final independent physical readback, cancellation and failure propagation.
- Existing ignored performance-only benchmarks; no test was newly ignored.

After the repair, phase fingerprints of all three fixtures matched both between
platforms and against the original macOS baseline, including the initial
eigenpairs and refined eigenpairs. The diagnostic probe was removed after this
comparison; only small regression tests remain in the source tree.

## Verification

The new regression tests pin eight minimal cross-platform rounding cases and
check sign/order symmetry. They separately validate 4,096 deterministic input
pairs against double-double square-root refinement (within one ULP for this
sample), and verify binary scaling at exponents -900, -500, 0, 500 and 900.
Exact 3-4-5 cases include subnormal and very large finite magnitudes.

| Check | Observed result |
| --- | --- |
| macOS modal-math unit tests, including new scalar regressions | 35 passed |
| Linux original coherent-phase and handoff failures | Both passed unchanged |
| All 12 solver numerical failures listed in the upstream CI run | Passed unchanged in Linux Debug and Release |
| Linux original three CLI failures | 3 passed unchanged |
| Linux complete solver library suite (unoptimized, matching CI) | 762 passed, 0 failed, 34 existing performance-only ignores; 3,044.01 seconds |
| Linux release solver library suite | 762 passed, 0 failed, 34 existing performance-only ignores; 243.82 seconds |
| Linux full modal CLI suite | 39 passed, 0 ignored; 164.97 seconds |
| macOS full modal CLI suite | 39 passed, 0 ignored; 174.15 seconds |
| Linux 11 modal integration groups | 112 passed, 0 failed, 0 ignored |
| macOS 11 modal integration groups | 112 passed, 0 failed, 0 ignored |
| Clippy (solver library and tests, `-D warnings`) | Passed |
| Workspace formatting and diff whitespace checks | Passed |

Reproduction from `workers/rust` with Rust 1.88.0:

```text
cargo test --locked -p kyuubiki-solver --lib -- --test-threads=4
cargo test --locked --release -p kyuubiki-solver --lib -- --test-threads=4
cargo test --locked -p kyuubiki-cli --test modal_spectrum_operator
cargo test --locked -p kyuubiki-solver --lib modal_math
```

The integration groups are `modal_assembly_reliability`, `modal_chain_reliability`,
`modal_cluster_reliability`, `modal_connected_reliability`, `modal_frame_2d_review`,
`modal_frame_3d_review`, `modal_frame_input_reliability`, `modal_frame_sanity_regression`,
`modal_mass_scaling_reliability`, `modal_shape_reliability` and
`modal_spectrum_reliability`. Select each with `--test` and use `--test-threads=2`.
Reported timings are regression-run observations under concurrent load, not a
paired performance benchmark.

Passing this regression does not establish universal convergence for extreme
models or certify every other platform-dependent operation in the solver.
Previously unresolved tiny-scale candidates must still fail their unchanged
residual gate rather than publishing an unchecked mode.
