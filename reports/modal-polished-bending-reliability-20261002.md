# Bounded Residual Polish for Bending Spectra

Date: 2026-10-02
Version: daji 3.4.3 working tree
Scope: local macOS, Rust Solver and in-process Rust headless plan-to-Engine execution

This is bounded residual-polished bending validation, not general modal qualification.

## Reproduced Boundary

The previous [compensated-residual work](modal-compensated-bending-reliability-20261002.md)
resolved the 80-element fixture, but 96, 100 and 128 elements still exhausted
four inverse corrections without meeting the `1e-8` relative residual gate.
Those historical measurements remain unchanged in that report.

Instrumentation showed that unit normalization could undo part of a correction:
the 80-element first corrected vector had relative residual `5.932287e-9`
before normalization and `8.607011e-9` afterward. At 128 elements, the first
correction changed from `4.377869e-8` to `5.543536e-8` across normalization.
This was one rounding source, not the whole problem. A binary-scaling-only
trial still rejected 96/100-element unit-coordinate cases at `1.423007e-8`
and `1.550901e-8`. No tolerance was relaxed to admit those trials.

## Numerical Change

The operator-local implementation remains in
`workers/rust/crates/solver/src/modal_frame_refinement.rs`. Engine dispatch,
task representation, SDK protocols and the ordinary sparse single-mode path
are unchanged. No runtime dependency, global cache or new dense matrix is added.

Internal vectors now use power-of-two scaling into a norm range of `[0.5, 1]`,
rather than division by an arbitrary norm after every correction. This avoids
unnecessary normal-range rounding while bounding Rayleigh products for large
representable eigenvalues. Subnormal input norms do not require an overflowing
reciprocal. Non-finite scaling and loss of a nonzero component to zero fail
explicitly; binary64 underflow and rounding limits are not removed.

Two-pass modified Gram-Schmidt divides each projection by the actual previous
vector norm squared. The Rayleigh quotient likewise divides by `v^T v`.
The external shape contract is unchanged: returned physical shapes remain
unit Euclidean vectors, with restrained DOFs zero.

After each existing inverse residual correction, an unresolved mode may try
exactly one additional residual-minimizing candidate:

```text
r = A v - lambda v
d = r / ||r||
p = (A - lambda I) d
tau = ||r|| (d^T p) / (p^T p)
candidate = v - tau d
```

The implementation rescales `p` before dot products to avoid unnecessary
overflow and underflow. It then orthogonalizes the candidate, recomputes its
Rayleigh quotient and measures the physical operator's relative residual
again. The candidate is committed only if that measured residual is strictly
smaller. Already-converged vectors skip this work; zero/nonrepresentable steps
and non-improving candidates leave the seed unchanged, still subject to the
original residual gate. Invalid operator products or eigenpairs and cancellation
remain errors, not partial successful spectra.

This is a bounded Richardson-style residual-minimization step, not a new
eigensolver or a proof of convergence. For background, PETSc documents
[self-scaled Richardson residual minimization](https://petsc.org/release/manualpages/KSP/KSPRichardsonSetSelfScale/).
Our implementation uses the current shifted physical modal operator and an
explicit post-orthogonalization acceptance check; it does not import PETSc or
claim equivalence to its full solver.

The four-inverse-correction limit, positive-root check, seed-residual spectral
neighborhood, independent-direction guard and final `1e-8` gate remain intact.
Each candidate adds at most two compensated operator products and linear-size
work vectors, with no hidden retry loop. The prepared inverse is still created
at most once per request. This adds bounded work; it is not a claim of unchanged
runtime or a performance benchmark.

## Independent References

The 2D fixtures have 96/100 uniform elements, 97/101 nodes and 192/200 free
bending DOFs. Axial DOFs and the root are fixed. The element stiffness, lumped
mass and independent banded `K - lambda M` inertia construction are the same
as the [80-element reference](modal-compensated-bending-reliability-20261002.md).

Offline inertia bisection used 150 steps on `[0, 192]`, separately at 80 and
120 decimal digits. Both precisions agree on these binary64 reference values.
The shipped native tests independently reconstruct the physical banded matrix
and check inertia at relative offsets `+/-1e-6` around every selected root.
There is no Python or arbitrary-precision runtime dependency.

| One-Based Mode | 96 Elements | 100 Elements |
| --- | --- | --- |
| 1 | `1.4553085447431156e-7` | `1.236075027143164e-7` |
| 2 | `5.712730198911023e-6` | `4.852330686025576e-6` |
| 3 | `4.47606463734338e-5` | `3.8021072825043554e-5` |
| 4 | `1.7174163867530644e-4` | `1.458920056234989e-4` |
| 5 | `4.688419676264506e-4` | `3.983054168160999e-4` |
| 6 | `1.045002999787545e-3` | `8.878653418209562e-4` |
| 10 | `9.241836936192287e-3` | `7.85608992786169e-3` |
| 20 | `1.5949592609683072e-1` | `1.3587583669754427e-1` |
| Last (192 / 200) | `1.5853323061113574e2` | `1.5853323061113574e2` |

Values are discrete FEM eigenvalues in `rad^2/s^2`, not continuum beam
frequencies or experimental measurements. Selected returned roots must agree
within `2e-8`; the independent reference does not certify every spectral index
for arbitrary inputs.

## Retained Coverage

- Public 2D requests return 6, 20 and all 192/200 modes at unit coordinates.
  Tests check sorted positive roots, selected low/high/last references, shape
  size, fixed zeros, unit norm, period conversion and pairwise mass orthogonality.
- Both model sizes request 20 modes at lengths `1`, `1e14` and `1e-10`, with
  `E=L^3`, `rho=1/L`, reversed node numbering, member ordering and endpoints.
- A 100-element spatial beam has 400 free bending DOFs. Six requested modes
  form three repeated bending pairs at all three coordinate scales. Tests
  verify multiplicity and mass orthogonality, not an arbitrary preferred basis.
- Rust headless tests run the actual plan-to-Engine operator route for 2D
  low/high modes and 3D repeated modes at those coordinate scales. They check
  physical output sizes, fixed zeros, finite unit shapes and period conversion.
- Cancellation inside the added compensated product propagates before the
  solve returns; fresh Solver and headless requests replay successfully. A
  direct unit test also verifies that an interrupted candidate leaves its seed
  intact. The public 100-element solve prepares exactly one dense inverse.
- Seven private tests check normal/subnormal binary scaling, component loss,
  nonunit projections, extreme Rayleigh roots, common operator scales, unchanged
  converged/non-improving seeds, cancellation and the zero/one-correction budget.

The earlier 66/80-element cases remain regression inputs. Their existing
reports are historical snapshots, not rewritten to imply that they originally
covered the larger fixtures.

## Remaining Boundary

The 128-element 2D six-mode fixture still cannot satisfy the original residual
gate within four inverse corrections. Both Solver and headless tests retain
this explicit refusal and then run a valid 80-element replay. It is safe-failure
coverage, **not** successful 128-element modal coverage.

Element counts here are fixture sizes, not universal limits. Geometry,
conditioning, topology and material contrast also matter. This work does not
qualify arbitrary 3D frames, a full 400-mode spatial spectrum, free-free modes,
damping, nonlinear dynamics, large complete spectra, installed-Agent recovery,
Linux execution or general industrial accuracy.

## Verification

Fifteen new retained tests: seven private numerical tests, five public Solver
tests and three in-process Rust headless tests.

| Check | Result |
| --- | --- |
| Private refinement target | 12 passed |
| Mass-coordinate / complete-bending public target | 18 passed |
| Rust headless modal target | 32 passed |
| Strict Solver all-target and CLI modal Clippy | Passed, warnings denied |
| Full Solver suite | 1262 passed, 0 failed, 9 existing ignored; 186 result groups including doc tests |
| Executed modal validation profile | 23 commands, 204 passed, 0 failed; artifact `ok=true` |
| Documentation / tensor / organization gates | Passed, including operator-validation and tensor self-tests |
| Formatting, diff check, source fingerprints | Passed; all ten recorded fingerprints verified |

The nine default ignored tests are eight opt-in microbenchmarks and one retained
postprocess physical comparison. No ignored test was added. Profile tests
overlap the full Solver and headless suites; these totals are not additive.

The tensor remains globally `blocked`: zero structural gaps, four maturity
gaps, sixteen evidence-grade gaps and eleven P0 gaps. This bounded numerical
and in-process recovery evidence does not close those project-wide gaps.

The machine-readable local artifact is intentionally ignored:
`tmp/modal-polished-bending-validation.json`. Run the profile from the repository
root with the native runner:

```text
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-polished-bending-validation.json
```

## Source Fingerprints

SHA-256 at the start of the final full-suite run. Paths are repository-relative;
no local machine configuration or credentials are retained.

```text
c21f16bb639d395c2b331dcbc573730d636894e8ee82acb828456c0530f575e7  workers/rust/crates/solver/src/modal_frame_refinement.rs
59864990618964ce29f2ae765d7e6b50bcc9bc69845447750fa10f6fed0e9ba3  workers/rust/crates/solver/src/modal_frame_refinement_tests.rs
30feeab4df4abe970e5f96d26588b7cb7edc4b74274cd74ff3abadf439b1537c  workers/rust/crates/solver/src/modal_frame_polish_tests.rs
a4ea1b4f2cea5dc63cd7365a522e8b77bcacbf0f896920460cdebddc170d9590  workers/rust/crates/solver/src/modal_sparse_product.rs
10c23f9471530d342d943c05c410a0c66222f403dd51917b1855e8ffc416b4df  workers/rust/crates/solver/src/modal_frame_spectrum.rs
deae071a341d1d7a0f08ed6cbd63e4078485931acc8712c92b9f259388c7e33e  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/polished.rs
4a94c17e8b07d7da9c506a44b271c94a2a2c3e68715b1526119888ada0dc578e  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/complete.rs
394cd4a555551166f35d55a9d675bc662504de9491f62d9bdbc37b489cabf087  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/spatial.rs
86fbefdd339c6480558c2e80a9ec021950285a69cf61d9f35ac91fbf87963636  workers/rust/crates/solver/tests/modal_mass_scaling_reliability/reference.rs
b5b93de2f9a6d1a4e8ff8728670a07f200e77e7a892ca525894bb8782378ec17  workers/rust/crates/cli/tests/modal_spectrum_operator/polished_bending.rs
```
