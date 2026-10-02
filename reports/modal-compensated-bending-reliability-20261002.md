# Compensated Residuals for Longer Bending Spectra

Date: 2026-10-02
Version: daji 3.4.3 working tree
Scope: local macOS, Rust Solver and Rust headless plan-to-Engine execution

This is bounded compensated bending validation, not general modal qualification.

## Reproduced Failure

The previous complete-bending regression covered a 66-element cantilever.
Increasing that fixture to 80 elements made the six-mode request fail:

```text
modal dense mode 0 refinement did not converge within 4 steps (relative=1.775746e-8)
```

The original relative residual limit is `1e-8`; the correction budget is four.
Changing inverse iteration to residual correction alone still failed at
`1.315672e-8`. Neither threshold nor budget was changed to admit these results.

The new implementation retains the physical stiffness and the same stored
inverse-square-root mass factors, but evaluates their products with compensated
arithmetic on the bounded dense-spectrum refinement and validation path.
The ordinary sparse matrix-vector product and sparse single-mode path are unchanged.

## Implementation

`workers/rust/crates/solver/src/modal_sparse_product.rs` preserves low-order
roundoff from input mass scaling, individual stiffness products and row sums.
Fused multiply-add recovers product roundoff; compensated summation retains
small terms otherwise lost between large cancelling terms. Output mass scaling
also combines the high and low parts before returning a single `f64` value.

The arithmetic approach follows the product-error and compensated-dot-product
techniques described by [Ogita, Rump and Oishi, Accurate Sum and Dot Product](https://www.tuhh.de/ti3/paper/rump/OgRuOi05.pdf).
This implementation does not claim an interval enclosure or an exact residual;
underflow, overflow and final `f64` rounding remain limitations.

The normalized prepared inverse is still reused, but the update is now
`v <- v - A_factor^-1 (A_physical v - lambda v)`, followed by the existing
two-pass orthogonalization and Rayleigh update. In exact arithmetic this is
inverse iteration up to scaling. In floating point it corrects the physical
operator's residual instead of repeatedly replacing the seed using the
separately rounded factor matrix.

The original spectral-neighborhood guard, independent-direction guard,
positive-eigenvalue check and final `1e-8` relative residual gate remain.
Compensated work is limited to the existing complete-spectrum path, capped at
4096 free DOFs. Its temporary vectors use linear memory; no new dense matrix,
runtime dependency, Engine contract, SDK protocol or global cache was added.

Cancellation is checked during input scaling, output rows and wide-row chunks.
Invalid dimensions, non-finite inputs and non-finite intermediates are explicit
errors. A cancelled or unresolved solve cannot return a partial spectrum.

## Independent Reference

The retained fixture has 80 unit elements, 81 nodes and 160 bending DOFs.
Each element uses `E=I=A=rho=L=1`, with the root fixed and every axial DOF fixed.
The independent physical bending block is:

```text
 12   6  -12   6
  6   4   -6   2
-12  -6   12  -6
  6   2   -6   4
```

The interior lumped masses are `1` and `1/12`; both tip values are halved.
An independent banded LDL inertia count of `K - lambda M`, with half-bandwidth
three, brackets each selected eigenvalue. Offline bisection used 150 steps on
`[0, 192]`, independently repeated at 80 and 120 decimal digits. Both runs
agree on the retained binary64 reference constants below. No Python dependency
or arbitrary-precision library is introduced into the product or regression tests.

| One-Based Mode | Eigenvalue, rad^2/s^2 |
| --- | --- |
| 1 | `3.017539808750544e-7` |
| 2 | `1.1842589597842897e-5` |
| 3 | `9.276402725565059e-5` |
| 4 | `3.557971963900987e-4` |
| 5 | `9.708758245053636e-4` |
| 6 | `2.1628755568757146e-3` |
| 10 | `1.9074331222057785e-2` |
| 20 | `3.252305299727169e-1` |
| 160 | `1.5853323061113574e2` |

Native tests independently reconstruct the physical banded matrix and check
inertia on both sides of each root at relative distance `1e-6`. Returned selected
roots must agree within `2e-8`; these are discrete FEM references, not continuum
beam frequencies or experimental measurements.

The public Solver suite checks requests for 6, 20 and all 160 modes, ordering,
positive frequencies, period conversion, restrained zeros, Euclidean shape
normalization and pairwise mass orthogonality. Coordinate lengths `1`, `1e14`
and `1e-10` are checked with `E=L^3`, `rho=1/L`, reversed node numbering,
reversed element order and reversed endpoints. The resulting congruent models
must retain the reference spectrum and mass-weighted independent directions.

Rust headless tests execute the actual plan, operator manifest and Engine path,
not a GUI mock. They check 20 modes at all three coordinate scales, cancellation
inside a compensated product after refinement starts, failure propagation and
fresh replay.

## Retained Limits

Longer chains are **not** promoted to successful numerical coverage. The local
exploratory six-mode sweep still rejected these unit-coordinate cases:

| Elements | Free Bending DOFs | Relative Residual After Four Corrections |
| --- | --- | --- |
| 96 | 192 | `2.328635e-8` |
| 100 | 200 | `1.508524e-8` |
| 128 | 256 | `5.551581e-8` |

The same lengths at coordinate scales `1e14` and `1e-10` also rejected unresolved
spectra. The 128-element refusal is retained through both Solver and headless
regressions, followed by a successful 80-element replay. This is evidence of
safe failure, not support for those larger solves. These observations are not
universal size limits: conditioning, geometry, material contrast and topology
also matter.

The next numerical issue remains longer flexural chains under the strict
mode-relative residual gate. This turn does not add free-free modes, damping,
geometric/material nonlinearity, arbitrary large complete spectra, Linux
execution or a globally certified eigenvalue-index method.

## Verification

Thirteen new retained tests: six arithmetic/cancellation unit tests, four public
Solver tests and three real headless-to-Engine tests.

| Check | Result |
| --- | --- |
| Compensated-product unit target | 6 passed |
| Mass-coordinate / complete-bending public target | 13 passed |
| Rust headless modal target | 29 passed |
| Strict Solver all-target and CLI modal Clippy | Passed, warnings denied |
| Full Solver suite | 1250 passed, 0 failed, 9 existing ignored; 186 result groups including doc tests |
| Executed modal validation profile | 23 commands, 189 passed, 0 failed; artifact `ok=true` |
| Documentation / tensor / organization gates | Passed, including operator-validation and tensor self-tests |
| Formatting, diff check, source fingerprints | Passed; all eight recorded fingerprints verified |

The nine default ignored tests are eight opt-in paired microbenchmarks and one
retained postprocess physical comparison. This turn added no ignored tests.
The tensor remains globally `blocked`: zero structural gaps, four maturity
gaps, sixteen evidence-grade gaps and eleven P0 gaps. The new bounded evidence
does not establish whole-project or general modal qualification.

The ignored machine-readable artifact is
`tmp/modal-compensated-bending-validation.json`. Reproduce the executed profile
from the repository root with the native runner:

```text
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-compensated-bending-validation.json
```

Runtime and test source were frozen before the full Solver run. Historical
reports retain their original measurements; this report is the follow-up,
not a retroactive replacement of the 66-element evidence.

## Source Fingerprints

SHA-256 at the start of the final full-suite run; paths are repository-relative.

```text
a4ea1b4f2cea5dc63cd7365a522e8b77bcacbf0f896920460cdebddc170d9590  workers/rust/crates/solver/src/modal_sparse_product.rs
94ebec8055c95233572c187b357d8e33eb55cb6827a4f8609a96bb09c38e2cb2  workers/rust/crates/solver/src/modal_sparse_product_tests.rs
12019eab6ea15725f07229089cb4e7f199a08fc570f1d859bd9cbd56a07451c7  workers/rust/crates/solver/src/modal_frame_refinement.rs
10c23f9471530d342d943c05c410a0c66222f403dd51917b1855e8ffc416b4df  workers/rust/crates/solver/src/modal_frame_spectrum.rs
501bcfbf1825207293f30ab6fa3e142523f1eca47897a36b19a7301713938470  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/longer.rs
2c8b0a2cfd453481a2755679fc513f7bd0c1431ef8dfb7989b7cfdb914d8a3c6  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/complete.rs
86fbefdd339c6480558c2e80a9ec021950285a69cf61d9f35ac91fbf87963636  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/reference.rs
fcad8e771e8455319687852b830303fb3cafdc41c301262f7ee1e0c81eaee822  workers/rust/crates/cli/tests/modal_spectrum_operator/longer_bending.rs
```
