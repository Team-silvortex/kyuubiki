# Modal Banded Inverse And Discrete Candidate Recovery

The separate [borrowed initializer and fresh cost follow-up](modal-banded-inverse-pipeline-cost-20261004.md)
preserves this report's numerical results while removing its initializer copy
and history from the measured path. Its own six-process timing and scoped work
ledger do not qualify production admission or whole-pipeline budgets.

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only six-fixture candidate recovery, not public modal admission or release qualification.

The previous [normalized grid comparison](modal-grid-normalized-reliability-20261004.md)
recovered five of six 128-element graded/layered fixtures. Its graded-large
input remained blocked even by a separate [wide QR beam](modal-grid-wide-beam-reliability-20261004.md).
This follow-up retains the actual four-step production-polishing best pair,
then uses four bounded wide banded inverse iterations to propose a different
direction before the existing normalized and physical discrete portfolios.

All six candidate pipelines now pass both stages and independent physical
JSON readback. The formerly blocked graded-large case publishes a candidate
with independent relative residual `3.961311840787512e-9`, below the unchanged
`1e-8` gate. This establishes a sampled representable solution, not a proof
of global optimality or an operational production recovery path.

All new code is below the existing test-only registration. No production
algorithm, tolerance, mode/dimension admission, Engine/Agent contract, dependency,
public API or retry budget changes. The public Solver is executed on all six
inputs in the new test and still explicitly rejects normalized recovery.

## Separate Precision From Representation

The retained fixtures have 256 active bending coordinates, with graded or
layered material parameters at common scales `1`, `1e14`, `1e-10`. No new
material dataset or geometry is added. The eigenvalue stays frozen through
all proposals. The existing separate banded inertia algorithm brackets the
first root to relative width below `1e-14`, with the frozen root differing
from its midpoint by less than `1e-12`. This is not interval certification.

The reference reconstructs the unshifted wide normalized operator by adding
back that root to the diagonal. It does not substitute a reference eigenvalue.
A symmetric positive-definite bandwidth-three LDL factor uses power-of-two
scaling and stores only three lower entries per row plus its diagonal. One
factor is reused for exactly four inverse iterations of the retained seed.
Wide iterates are scaled to the retained anchor; final f64 proposals preserve
its bits. The continuous wide direction is not itself an accepted result.

At the fourth step, the shifted wide residual divided by frozen root times
direction norm is below `1e-14` on every fixture. But all 24 nearest-rounded
directions across the four steps fail the actual compensated operator's
`1e-8` gate and are worse than the original retained seed. This is evidence
that greater factor precision or nearest rounding alone is not this sample's
solution. It is not proof that all floating-point directions must fail.

The fourth-step rounded direction instead initializes the existing canonical
discrete portfolio. Normalized admission and physical publication each retain
their existing maximum three orders and 19 true-operator certificates. A
fresh final certificate remains mandatory. Failed baseline portfolio results
are only comparison data, never appended as runtime retries.

## Six Retained Results

Display values are rounded, not altered acceptance gates. Continuous residuals
use the same wide shifted matrix as the proposal reference; they do not replace
the real compensated operator or independently reassembled physical check.

| Profile | Scale | Fourth-step continuous residual | Nearest-rounded residual | Accepted normalized residual | Independent physical JSON residual | Normalized / physical checks |
| --- | --- | ---: | ---: | ---: | ---: | --- |
| graded | 1 | 1.479607e-18 | 9.718614e-8 | 2.667466e-9 | 7.705861e-9 | 12 / 4 |
| graded | 1e14 | 1.018029e-15 | 9.379865e-8 | 4.047269e-9 | 3.961312e-9 | 12 / 11 |
| graded | 1e-10 | 1.077417e-16 | 8.856505e-8 | 3.771156e-9 | 7.078472e-9 | 12 / 4 |
| layered | 1 | 5.861082e-18 | 1.028602e-7 | 5.286986e-9 | 8.333676e-9 | 12 / 4 |
| layered | 1e14 | 5.406650e-17 | 8.980704e-8 | 7.471159e-9 | 8.981978e-9 | 4 / 4 |
| layered | 1e-10 | 6.425925e-17 | 1.063850e-7 | 8.675553e-9 | 8.589527e-9 | 4 / 4 |

```text
banded inverse fixtures=6 nearest_rounding_rejections=24 direct_grid_successes=5 two_stage_successes=6
banded inverse amplitude_sign_controls=24 two_stage_successes=24
```

The old direct-grid five-of-six boundary is separately asserted, alongside the
new six-of-six candidate result. Twenty-four controls apply amplitudes `-2`,
`-1`, `0.5`, `2` to these same seeds. Rounded directions and accepted internal
vectors retain baseline bits after removing amplitude; physical shapes retain
bits after sign restoration. Internal and independent physical residual bits
and certificate counts match. These transformations are not 24 more physical
datasets or independent renumbered requests.

Physical publication still uses original positive masses and the strict
`1e-10` unit-norm gate. Final shapes and frozen eigenvalues are serialized,
read back bitwise, and checked by independent stiffness/mass reconstruction
from rounded input geometry, without production assembly/product helpers.
The original retained seed and root remain unchanged.

## Bounds And Failure Recovery

The new reference admits sizes 2 through 256 and steps 1 through 4 only.
Square finite bounded inputs, symmetric bandwidth, positive finite pivots and
bounded factor coefficients are mandatory. Invalid shapes, asymmetry,
out-of-band entries, zero/negative pivots, nonfinite parts, unresolved matrix
scaling, right-hand-side overflow, invalid seeds and excess iterations reject.
Overflow and nonzero-entry loss are checked before risky wide scaling.

The factor's numeric payload is `64*n` bytes, plus constant vector headers and
the scale. Each returned wide direction has `16*n` bytes, at most four, and
substitution uses bounded linear temporary storage. Validation and caller
matrix reconstruction still visit/copy dense `n*n` input, and the existing
two-stage discrete portfolios retain their own dense preparation and cost.
This is structural local boundedness, not a complete allocation bound, process
RSS measurement, timing guarantee or production budget qualification.

Independent known-solution controls use diagonally dominant bandwidth-three
matrices of sizes 2, 4, 65, 256, binary matrix scales `2^-100`, `1`, `2^100`,
and both RHS signs. All 24 solves round to exact known solution bits, with
wide solution error below `1e-25`. A separate two-coordinate closed form checks
`[[2,1],[1,3]] * [0.2,0.6] = [1,2]` within wide arithmetic.

Six observer-verified cancellation points cover preparation, factorization,
forward/diagonal/back substitution and final iteration completion. Fresh replay
retains both wide parts, not just rounded outputs; caller input is unchanged.
The actual graded-large recovered candidate separately cancels at the fourth
iteration and final physical restoration. An injected final normalized
certificate fault rejects after earlier candidate checks; fresh replay matches
original normalized/physical bits and independent residual.

## Verification And Remaining Work

| Check | Result |
| --- | --- |
| Optimized focused factor/candidate suite | 6 passed, 0 failed |
| Optimized full Solver unit library | 670 passed, 0 failed; 16 explicit benchmark/reference cases ignored |
| Debug focused factor/candidate suite | 6 passed, 0 failed; overlapping optimized cases, not extra fixtures |
| Tensor source regression | 21 passed, 0 failed; new test-only proof cannot bind production recovery |
| Solver/CLI and tensor runner all-target Clippy | Passed with `-D warnings` |
| Rust formatting and whitespace | Passed |
| Operator validation registry and self-test | Passed; 59 profiles; registry-only `executed=false` |
| Tensor validator and self-test | Structure and command passed; 4 dimension-presence, 19 evidence-grade and 14 P0 gaps remain; readiness blocked |
| Documentation inventory and book | Passed; 26 HTML files; development/shipping 3.4.5 |
| Project organization audit and self-test | Passed; source <=800, docs <=2000; tracked debt 0 |

The complete library run includes prior public request-reassembly controls.
Integration/Headless/live Agent suites and the 16 explicit ignored cases were
not freshly executed in this follow-up. Recorded suite wall times are not a
benchmark or a production performance claim.

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib --release modal_banded_inverse_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib modal_banded_inverse_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The tensor adds a verified validation/recovery claim for this test-only
candidate pipeline. It binds to the existing candidate research scope, not
the heterogeneous production scope. Retained claims total 226 (224 proven,
two partial), with 58/77 coordinates and 10/32 scenarios at target. Public
modal admission, complete-pipeline budgets, independently rebuilt candidate
requests, arbitrary topology, multimode, actual Agent/Headless study journeys,
remote execution and installed qualification remain separate work.

The next production step must replace or integrate a bounded recovery strategy
with cumulative work/allocation accounting and fault isolation; it must not
simply stack this reference after the old search. This turn adds no runtime
fallback, package, installation, version or release claim.
