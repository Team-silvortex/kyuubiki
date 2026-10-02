# Complete Bending Spectrum Reliability: 2026-10-02

## Scope And Reproduction

Base revision: `99e494d1`, daji 3.4.3, with the preceding uncommitted modal
reliability overlay. This follows the explicitly unresolved complete-spectrum
boundary in the [mass-inverse report](modal-mass-inverse-reliability-20261002.md).
The earlier report remains a historical snapshot, not a current claim that
this particular rejection is still unresolved.

A connected 66-element 2D cantilever has 132 free bending DOFs. At unit
segment length, area/inertia/modulus/density one, requesting six modes
reproduced the failure before refinement was added:

```text
modal frame mode 0 failed its relative residual check
(relative=1.981067e-7, tolerance=1.000000e-8)
```

Jacobi's transformed matrix had met its off-diagonal stopping rule, but the
recovered low mode did not satisfy the original operator at the required
relative accuracy. A trial using a shorter diagonal rotation update reduced
this only to `1.765724e-7`; that trial was reverted. Neither Jacobi's
stopping tolerance nor the final residual gate was weakened.

## Bounded Refinement

Only dense-derived, requested frame modes enter the new refinement stage.
Modes already meeting the original residual gate keep their values and basis
and do not prepare an inverse. A failing mode reuses the preceding native
mass-normalized inverse implementation, with one lazily prepared system per
request. Each mode has at most four inverse correction steps. Zero/one-step
budget regressions prove that unresolved cases return errors rather than
extending the budget silently.

Each correction normalizes the candidate, applies two-pass modified
Gram-Schmidt against previously accepted directions and recomputes its
Rayleigh value using a compensated dot product. Later modes are
reorthogonalized when an earlier direction changed. Results are sorted
again after correction, then pass the existing original-operator residual
check. Invalid/nonpositive values, lost independent directions, excessive
root drift, unresolved residuals and cancellation remain errors.

The drift guard bounds the relative eigenvalue change by four times the
larger of the initial relative residual and acceptance tolerance. It is a
conservative local guard, not a rigorous spectral-index or unique-branch
certificate. Closely repeated eigenspaces may legitimately change basis.

Inverse iteration and reorthogonalization are established numerical tools,
but their cluster/accuracy limitations still apply; see the primary
[LAPACK symmetric eigensolver comparison](https://www.netlib.org/lapack/lawnspdf/lawn183.pdf).
This implementation is a bounded native correction of the existing frame
results, not an implementation of LAPACK's full eigensolver stack or a new
LAPACK/runtime dependency.

The sparse single-mode algorithm is unchanged. The 128-DOF single-mode dense
fallback, 1024-DOF inner dense limit, 4096-DOF complete-spectrum bound and
40-sweep Jacobi budget are unchanged. Refinement adds explicit bounded work
after Jacobi; this is an accuracy/availability repair, not a claimed speedup.
Frame validation errors now include the measured relative residual and
required tolerance. No Engine dispatch, SDK schema, mass physics or GUI
behavior is changed.

`ModalIteration` can now also describe a bounded dense-result correction.
That stage alone does not prove a sparse outer solve; sparse-path evidence
must also check the request/size and actual factor/iteration stages.

## Independent Discrete Reference

The reference is the same discrete beam model, not a continuum frequency
mistaken for the lumped-mass discretization. An independently assembled
element matrix, in `[y_i, theta_i, y_j, theta_j]` order, is:

```text
K_element = [[12, 6,-12, 6],
             [ 6, 4, -6, 2],
             [-12,-6,12,-6],
             [ 6, 2, -6, 4]]
M_node interior = diag(1,1/12)
M_node free tip = diag(1/2,1/24)
```

The root translation/rotation are removed. Banded LDL inertia of
`K-lambda*M`, with scalar half-bandwidth three, brackets the generalized
eigenvalues without forming a mass-normalized matrix or using Jacobi.
Reference constants were calculated using standard-library decimal
arithmetic at 80 digits and rechecked at 120 digits, with 150 bisections
from `[0,192]`; both calculations agreed on all stored f64 constants.
Arithmetic precision is not a claim of 80 correct digits after 150 steps.
No Python code or dependency is added to the runtime or test runner.

| Mode | Discrete eigenvalue, rad^2/s^2 |
| --- | --- |
| 1 | `6.513218799205997e-7` |
| 2 | `2.5553097863515013e-5` |
| 3 | `2.0007498276160384e-4` |
| 4 | `7.669658416882003e-4` |
| 5 | `2.0914454533768654e-3` |
| 6 | `4.655568785074286e-3` |

A separate native Rust inertia test reproduces the independent assembly and
requires exactly `i`/`i+1` negative pivots on either side of root `i`, at
relative brackets `1 +/- 1e-6`. Production roots are compared with the
high-precision constants within `2e-8` relative error; their production
original-operator residual must independently satisfy `1e-8`. These are
different checks, not a relaxed residual gate.

Public regressions cover requests for 2, 6 and all 132 planar modes,
default/owned requests, reversed node/member/endpoint ordering and segment
lengths `1`, `1e14`, `1e-10` with `E=L^3` and density `1/L`. The normalized
discrete operator is invariant under these coordinate scalings. Shapes
have correct dimensions, finite values, exact restrained zeros, unit
Euclidean norm and pairwise mass-weighted overlap below `1e-10`.

The spatial fixture has 264 free DOFs and two identical bending planes.
Its first six modes retain the first three reference frequencies twice,
without duplicated directions, at all three coordinate scales. This does
not prescribe arbitrary basis directions inside a repeated eigenspace.
The extreme coordinate tests are numerical probes, not material advice.

## Execution And Recovery

Thirteen new regressions cover five numerical, six public-operator and two
Rust headless cases. A real execution batch, plan, bridge route and
in-process Engine execute the six-mode model. Result counts, frequencies,
shapes, restraints and period consistency are checked after serialization.
This is not a GUI mock, remote deployment or Python/Elixir verification.

Cancellation is injected inside dense factorization, substitution, inverse
refinement and orthogonalization. The operation must fail inside the
observer scope and must not publish a partial spectrum. Valid requests then
replay successfully. The 132-DOF fixture prepares exactly one factor for
the correction stage; numerical tests separately prove no preparation for
already accurate inputs and preservation of repeated independent directions.

## Validation

| Lane | Result |
| --- | --- |
| macOS ARM64 mass-coordinate/complete-bending public target | 9 passed, 0 failed |
| macOS ARM64 modal Rust headless target | 26 passed, 0 failed |
| macOS ARM64 numerical refinement target | 5 passed, 0 failed |
| macOS ARM64 Solver library | 444 passed, 0 failed, 9 ignored |
| macOS ARM64 complete modal profile, twenty-two commands | 176 passed, 0 failed |
| macOS ARM64 full Solver suite, library/integration/doc harnesses | 1,240 passed, 0 failed, 9 ignored |
| Strict Clippy, formatting and repository gates | Passed |

The source-frozen full Solver run exited successfully with 186 completed
result groups, ending with a zero-test doc-test harness. The complete modal
profile also exited successfully; its generated artifact records 22 commands
and `ok=true`. Strict Clippy passed with warnings denied for all Solver
targets and the modal CLI target. Formatting, operator-validation and tensor
self-tests, documentation book/inventory, whitespace and the
source-800/document-2000 organization checks passed. The guide remains at
1999 newline lines. The nine ignored tests are the existing eight opt-in
benchmarks and one retained physical-reference comparison; none were added
to bypass this defect. Overlapping lanes must not be added into coverage.
Historical reports keep their own results and hashes.

Tensor structure and command checks pass, but global status remains blocked:
0 structural gaps, 4 maturity gaps, 16 evidence-grade gaps and 11 P0 gaps.
This bounded repair is not a claim of whole-project qualification.

Frozen code SHA-256 values, relative to `workers/rust/crates/`:

| File | SHA-256 |
| --- | --- |
| `solver/src/modal_frame_spectrum.rs` | `115ab9df2465cdd5d305fcbe6777a85ec57bc050e2536cd9e73ac7f82b663cd4` |
| `solver/src/modal_frame_refinement.rs` | `97d6db47b8834e0dd1148036c4966898236ecbe1211d491cbb33e9a40a1d1e4c` |
| `solver/src/modal_frame_refinement_tests.rs` | `2882bf291716bfe28e3d66c563a1705071149b2e8f8d871a2264c203a49e82f5` |
| `solver/tests/modal_mass_scaling_reliability.rs` | `ab883d71cfe3b625798a180d5196bcafd02409ec9142af4ebaf9443504ed5572` |
| `solver/tests/modal_mass_scaling_reliability/complete.rs` | `2fe7796a00d4be50dbf5c62a24869a9fce6631f575578a38bd64d7573e0517a4` |
| `solver/tests/modal_mass_scaling_reliability/reference.rs` | `d31701d1f74631946271c4f44801991b295a911c3980264f68f7f03f86937927` |
| `solver/tests/modal_mass_scaling_reliability/spatial.rs` | `a4be9df7380efe5af083a3fc1fa6a98e8bfc36bf91459215fba1b06861023479` |
| `cli/tests/modal_spectrum_operator/mass_scaling.rs` | `792ba318217765ff627ce9f49ad41dd94fcd99fcb9975acc4d99af8e856ee41f` |
| `cli/tests/modal_spectrum_operator/complete_bending.rs` | `835f49f6224f69d3ff21367ca1a984ba91bb567eff1344b7ab1ba40f87ed8d9e` |

## Boundaries

This is bounded complete-bending refinement validation, not general modal qualification.
The specific 66-element rejection is repaired without relaxing acceptance.
This does not qualify arbitrary ill-conditioned complete spectra, nonlinear
dynamics, experimental materials, very large full spectra or 1M modal
performance. Four correction steps can still fail on other difficult
models; such failure remains explicit. Residual and orthogonality checks do
not certify a globally correct modal index for every possible matrix.

Tensor evidence is registered only for headless/solver execution,
numerical validation and recovery, without promoting unrelated maturity
coordinates. No version change, Git commit/push, App rebuild, server access,
Linux run or installed GUI test is part of this round. All validation is
local macOS ARM64 with pinned Rust 1.88.0. Models are generated in tests;
execution evidence belongs in ignored `tmp/modal-complete-bending-validation.json`
and transient logs outside the repository.

Run from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --lib modal_frame_spectrum::refinement
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --test modal_mass_scaling_reliability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test modal_spectrum_operator
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver
cargo clippy --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-solver --all-targets -- -D warnings
env CARGO_NET_OFFLINE=true workers/rust/target/debug/kyuubiki-script-runner check-operator-validation --profile modal-frame-sanity --execute --out tmp/modal-complete-bending-validation.json
workers/rust/target/debug/kyuubiki-script-runner check-module-function-coverage-tensor
```
