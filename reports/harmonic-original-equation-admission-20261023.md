# Original Harmonic-Spring Complex-Equation Admission

Date: 2026-10-23

Version line: daji 3.6.x, after `0a65f2e8`. This extends the current
original-equation admission work to `solve.harmonic_spring_1d`; it does not
change the engine, TaskIR capabilities or operator input/output formats.

Follow-up: [frequency-product range recovery](harmonic-frequency-product-range-20261023.md)
also repairs coefficient construction before this gate. Validation totals below
record the gate-only snapshot; current shared test commands include later cases.

## Reproduced Failure

Four new negative controls failed before production changes. In particular,
a public three-node request returned a successful but locally unbalanced result:

```text
fixed root -- k=1e300 -- free middle -- k=1e-300 -- free tip
load[middle] = 0, load[tip] = 1, frequency = 0
old published u[middle] = 0, u[tip] approximately 1e300
```

The soft spring carried approximately one unit of force, while the grounded
hard spring carried zero. The middle equation was not balanced. The old checker
separately divided coefficients and incident displacements by their maxima.
The soft coefficient divided by `1e300` became zero, erasing its finite force
before the residual and denominator were computed. A zero equation scale then
produced zero relative error. The new gate rejects this real public solve with
frequency and free-row context; it does not claim to repair this extreme solve.

The other controls injected minimum-subnormal real/imaginary support drift and
non-finite values on an isolated fixed node into the private checker. Those
states also passed previously. These injected cases demonstrate missing defensive
contracts, not that a public request can directly supply an internal complex
solution or that ordinary previous harmonic results were all wrong.

## Contract And Implementation

Every frequency now validates the complete complex solution and canonical
reduction map against the original node constraints, independently of a reduced
solve succeeding. Dimensions must match; all real/imaginary values and applied
loads must be finite. Both components of a fixed displacement equal zero exactly;
signed zeros remain equal. Loaded supports remain valid because support equations
can carry reactions rather than satisfying zero-residual equilibrium.

Each original free equation is then checked as:

```text
(K - omega^2 M + i omega C) u = f
```

Two passes over original mass and element terms choose a binary exponent from
actual scalar products and the load. Mantissa/exponent decomposition avoids
forming overflowing or underflowing unscaled products. Product tails use fused
multiply-add, real/imaginary residuals use compensated sums, and normalization
must be reversible for each high and low product component. An unrepresentable
alignment returns an error rather than becoming a zero contribution.

The complex denominator retains the sum of original term magnitudes and load
magnitude, evaluated from the scaled scalar products. The existing free-count
tolerance remains `(128 * EPSILON * free_count).clamp(1e-12, 1e-8)`; it is not
relaxed to admit the counterexample. Diagonal mass/stiffness cancellation is
checked from the original separate terms, not a mutated factorization.

The family-specific checker is in
`workers/rust/crates/solver/src/harmonic_spring_constraint_validation.rs`.
It reuses the shared binary decomposition/restoration primitives without imposing
an SPD or real-only constraint policy on the complex operator. Both path and
dense solves call the same gate before node/element results are published.

Work remains O(nodes + elements) per frequency with O(free DOFs) gate storage.
The row accumulator uses 56 bytes per free row on the tested 64-bit platforms,
versus the old 40-byte scale/residual arrays. No original or doubled-real matrix,
additional factorization or per-element product cache is retained. Stronger
arithmetic adds CPU work; this report claims no speed or peak-memory improvement.

Complete-state entry/scans and final admission use `ConstraintValidate`; original
term scans retain `HarmonicResidual` cancellation checkpoints. Existing stage IDs
are unchanged. Failed or cancelled sweeps return errors, not partial successful
frequency arrays. Same-input replay does not require persistent mutable state.

## Added Regression Coverage

Thirteen new tests comprise nine solver unit cases and four engine integrations:

- The public erased-force regression and direct private original-row control.
- Minimum-subnormal fixed real and imaginary drift, isolated hidden NaN/infinity,
  incomplete/extra dimensions, omitted/duplicate/permuted/fixed/out-of-range maps.
- Loaded supports and signed zeros, plus a manufactured damped complex response
  at binary common scales `2^-600`, `1`, `2^600`. A wrong imaginary equation is
  rejected even when real equilibrium still holds.
- Signed `2^500`/`2^-500` stiffness/response contrast remains admissible; a wider
  `2^1000`/`2^-1000` case with unrepresentable self terms is conservatively rejected.
- Complete-state and final-admission cancellation, plus all six load/node/element
  scan phases at 1,024-item checkpoints in a 2,051-node restrained star, then replay.
- Serialized direct/workflow equality and one admission entry/completion for each
  sampled frequency on path and cyclic dense systems, including repeated frequencies.
- Bad-force rejection through both engine entries, second-frequency cancellation
  after an earlier frequency completed, and clean replay through both solver paths.
- Signed amplitude/phase closed forms through direct and workflow execution at
  the three common binary scales above.

The star is bounded cancellation evidence, not a large-scale performance certificate.

## Validation

Pinned Rust 1.88.0, offline locked dependencies and existing build directories:

| Final-source scope | macOS AArch64 passed | Linux x86_64 passed |
| --- | ---: | ---: |
| Solver library excluding `modal_frame_spectrum::` | 771 | 771 |
| Engine library | 711 | 711 |
| New engine harmonic integration target | 4 | 4 |
| Existing harmonic balance/reliability and dynamic closed-form targets | 23 | 23 |
| Non-overlapping total | 1,509 | 1,509 |

Each platform has zero final failures, ten ignored library tests and 292 filtered
modal cases. The thirteen new tests are included once, not added to the totals
again. Existing damped resonance, cycle modal decomposition, average power balance,
common scales from `1e-300` to `1e300`, pivoting, overflowing coefficient norms,
response publication limits and the 10,000-node shuffled path forest pass.

Local all-target Solver/Engine Clippy passes with warnings denied. Source/config
files remain below 800 lines. The remote run synchronized only four numerical
source/test files into the existing test tree, without deleting remote content,
deploying services, altering credentials or creating another build root.

Native operator metadata checks pass for 86 profiles (`executed=false`). All
three commands in the new profile also pass with `--execute`; their focused
reruns are not added to the totals above. Tensor structure/command checks pass
for 13 modules and 11 paradigms, with zero structural gaps, two maturity gaps,
23 evidence-grade gaps and 18 P0 gaps. Readiness remains `blocked`; this bounded
repair does not promote overall solver qualification. Project organization,
formatting/whitespace checks, the 26-file HTML book and documentation inventory
also pass. Small machine-readable reports remain in ignored `tmp/` storage.

## Reproduction

From the repository root:

```sh
cargo test --offline --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-solver -p kyuubiki-engine --lib \
  --test harmonic_constraint_admission --test harmonic_spring_balance \
  --test harmonic_spring_reliability --test dynamic_spring_closed_form \
  --no-fail-fast -- --skip modal_frame_spectrum:: --quiet
```

## Limits

This is an admission repair, not a higher-precision solver or proof of conditioning,
uniqueness, forward accuracy, constitutive validity or convergence. The gate uses
the validated original f64 element/mass data; it is not independent physical
assembly. It cannot recover a coefficient already lost while constructing dynamic
stiffness, nor certify arbitrary coefficient/unknown ranges. Conservative range
errors remain errors. Other complex/eigen formulations still need their own
contracts. Cancellation/replay does not establish durable process or power-loss
recovery. No application rebuild, version bump, staging or commit was performed.

This is bounded original harmonic-spring complex-equation admission, not general harmonic qualification, forward accuracy, independent physical assembly or durable restart.
