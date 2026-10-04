# Modal Wide Factor And Bounded Beam Diagnosis

Date: 2026-10-04
Source line: daji 3.4.5 working-tree overlay based on `431b2ac7`.
Platform: local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only factor/search diagnosis, not production modal recovery or public solver admission.

The [normalized candidate comparison](modal-grid-normalized-reliability-20261004.md)
retains five recovered fixture families and one graded-large rejection. This
follow-on compares factor precision and discrete search on those exact six
128-element assembled fixtures, each with 256 free coordinates. It does not
add experimental materials, independent geometry, new roots or production
fallbacks. Actual Jacobi pairs and the retained best failed four-step refinement
pairs are reconstructed through the existing fixture, then their eigenvalues
are frozen. Internal seeds are not replaced by wide-reference vectors or
renormalized to conceal their representation boundary.

All new code remains beneath the existing `cfg(test)` grid registration.
No production admission, Engine/operator contract, dependency, tolerance,
public API, version, installed application or runtime retry is changed.
The original public path still has the retained six-fixture rejection boundary.
Existing cost reports do not measure this new beam algorithm.

## Precision Isolation

Two canonical fixed-anchor orders are compared separately: reverse natural
order and ascending norm of the grid-scaled column. Three greedy factor paths
use the same seed, frozen anchor, integer radius `2^22`, actual operator
callback and maximum six certificates:

- Existing f64 Householder factor on rounded grid-scaled columns.
- Double-double Householder factor on those same rounded column inputs.
- Double-double factor retaining the low parts of the scaled direction matrix.

The latter two use scaled wide reflectors, wide triangular accumulation and
low-tail-aware integer decisions. Rank, resolved-range and cancellation checks
remain fail-closed. Analytic low-tail half-integer and exact compact triangular
controls prevent a precision flag from being only a label.

All three greedy paths give identical sampled residual, internal vector,
physical vector and certificate count. Each accepts six of the twelve
fixture/order combinations. Bit comparisons are asserted in addition to f64
equality. This is evidence against factor precision alone solving this sampled
boundary, not a universal statement about QR accuracy.

The shifted wide matrix applied to each original retained seed differs from
the actual compensated operator residual by `4.77e-17` to `5.02e-17`, divided
by the frozen eigenvalue times actual vector norm. The retained bound is
`1e-15`. Predicted seed residuals remain above `1e-8` and agree with the
real failing scale. This checks those seeds only, not every search candidate,
independent assembly or interval arithmetic.

## Four-State Search

A separate wide QR beam is one alternative factor/search run, not an extra
retry appended after the existing greedy or canonical portfolio. Reverse
triangular substitution expands each partial decision to its nearest bounded
integer and its two adjacent integers. At most twelve temporary partial states
are ranked by wide triangular prediction error, then reduced to four.
Exact-score tie breaking uses absolute decisions and RHS-oriented signs so it
does not favor a mode sign. This is a fixed-width heuristic, not exhaustive
search or a proof of a global integer optimum.

Every completed correction is applied once to the immutable original seed.
At most four proposals receive actual compensated-operator certificates;
the true residual, not the prediction score, chooses the retained candidate
including the original seed. A final fresh certificate is mandatory even
after a candidate passes. The anchor remains bitwise frozen. The six-check
cap covers initial, four proposal and final checks; no early successful
proposal can bypass later operator errors or final cancellation.

An independent two-coordinate exhaustive integer reference demonstrates a
real greedy barrier: columns `[1,0,0]` and `[4.4,1,0]` with RHS
`[2.49,0.49,0]` select greedy `[2,0]`, whereas the beam contains the better
bounded `[-2,1]`. This compact reference covers three row numberings, both
signs and binary column/RHS scales `2^-100`, `1`, `2^100`. Exact half-integer
score ties have a separate sign-reflection check. Neither proves global
optimality for the 256-coordinate fixtures.

## Retained Results

All three greedy backends share the greedy column below. The beam is a
separate one-factor attempt for the same order; accepted entries must also
pass physical unit norm and independent stiffness/mass reassembly after JSON
readback. All numeric rows show rounded display values, not changed gates.

| Profile | Scale | Order | Greedy best relative residual | Beam best relative residual | Beam independent physical JSON residual |
| --- | --- | --- | --- | --- | --- |
| graded | 1e0 | reverse | 1.511732e-8 | 1.066495e-8 | rejected internally |
| graded | 1e0 | ascending | 2.667466e-9 | 2.590543e-9 | 9.243483e-9 |
| graded | 1e14 | reverse | 5.144329e-8 | 5.144329e-8 | rejected internally |
| graded | 1e14 | ascending | 5.144329e-8 | 5.144329e-8 | rejected internally |
| graded | 1e-10 | reverse | 1.271301e-8 | 9.137181e-9 | 3.735993e-9 |
| graded | 1e-10 | ascending | 3.771156e-9 | 3.643144e-9 | 3.748081e-9 |
| layered | 1e0 | reverse | 1.045608e-8 | 9.052706e-9 | 7.013046e-9 |
| layered | 1e0 | ascending | 5.286986e-9 | 4.543047e-9 | 6.729587e-9 |
| layered | 1e14 | reverse | 7.471159e-9 | 8.022415e-9 | 7.199897e-9 |
| layered | 1e14 | ascending | 4.985585e-9 | 4.021065e-9 | 6.147297e-9 |
| layered | 1e-10 | reverse | 8.675553e-9 | 7.282428e-9 | 7.803890e-9 |
| layered | 1e-10 | ascending | 2.065619e-8 | 2.335045e-8 | rejected internally |

The beam accepts eight order proposals versus six per greedy backend, adding
graded-tiny/reverse and layered-unit/reverse successes. It still recovers only
five fixture families. It is not pointwise dominant: layered-large/reverse and
layered-tiny/ascending have worse best beam residuals than greedy. A fixed beam
can prune the greedy path, so it must not be described as a guaranteed upgrade.
The graded-large fixture remains rejected at `5.144329e-8` in both orders.

There are 48 precision/order fits: 26 accept both candidate stages and
independent JSON checks; 22 reject internally and never get a physical result.
Repeated transformation controls are separate from those 48 fits:

- Identity, reversed and shuffle-113 numbering with both signs: 72 beam runs,
  48 two-stage successes and 24 internal rejections.
- Binary seed amplitudes `0.5` and `2`: 24 beam runs, 16 two-stage successes
  and eight internal rejections.

All transformations retain the order-specific acceptance boundary, exact best
residual and six certificate count. Accepted internal vectors recover their
baseline bits after removing amplitude/sign; final unit physical vectors retain
baseline bits after sign restoration. These jointly permute fixed assembled
seeds and matrices: rebuilding a request after renumbering is still outside
scope. Multiple orders, signs, precision choices and amplitudes are not extra
physical datasets or additive public runtime coverage.

## Budgets And Failure Recovery

Preflight still admits only dimensions 2 through 256. The separate wide-grid
payload plan is `96*n*n + 2048*n` bytes, or `6,815,744` bytes at 256, below the
local 8 MiB proposal cap. Component-visit planning remains
`12*n*n*n + 2048*n*n`, or `335,544,320`, below 350 million. Four beam states and
twelve temporary partials are fixed constants, not user-scalable history.
These conservative local plans are not a whole canonical caller allocation
bound, process RSS measurement, double-double FLOP count or complete two-stage
production budget. Production's existing work/check budget is not raised.

Boundary tests cover invalid dimensions, radius, tolerance, ragged/rank-deficient
or nonfinite columns, malformed RHS, invalid certificate length/range/relative
value, and original operator faults at every one of the six callback positions.
A passing intermediate candidate followed by a failing final certificate
rejects. A passing seed is also freshly rechecked. Worse proposals retain the
seed and no new factor is built during search.

Cancellation is exercised during factor construction, triangular substitution,
vector update, final certificate checkpoint and final acceptance checkpoint.
Fresh replay uses unchanged retained seed/factor state and completes without
adopting cancelled proposals. Observer assertions enforce actual six-check
usage rather than trusting the declared constant.

A separate actual graded-tiny/reverse candidate is cancelled at the final
acceptance checkpoint after all six compensated-operator checks. Fresh replay
on the retained fit reproduces exact internal, physical and independent JSON
residual bits, with the original seed and eigenvalue unchanged. This real
candidate replay is not added to the transformation or fixture counts above.

## Replay And Remaining Work

From `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib triangular_grid_wide_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release triangular_grid_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib --release --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The validation registry carries the explicit wide comparison command. Registry
validation is not test execution. Scoped evidence is attached to
`runtime-engine-solver / validation` for numerical validation and recovery,
without production or release-maturity promotion.

Priorities remain graded-large recovery, bounded search structures that do
not simply stack retries, full production allocation/work qualification,
unequal-length downstream failures, independent renumbered assembly and
multimode validation. This turn adds no benchmark, remote Agent, installed
application, Headless or cross-platform qualification.

## Final Verification

| Check | Result |
| --- | --- |
| Optimized full Solver unit library | 658 passed, 0 failed, 16 explicit benchmark/reference tests ignored |
| Triangular-grid cases within that final library run | 42 passed, 0 failed, 7 explicit cost tests ignored |
| Debug precision/search controls and transformation checks | 8 passed, 0 failed |
| Debug actual late-cancellation replay, separate final invocation | 1 passed, 0 failed |
| Solver and CLI all-target Clippy | Passed with `-D warnings` |
| Rust formatting and whitespace | Passed |
| Operator validation registry and self-test | Passed; 59 profiles; `executed=false` denotes registry validation only |
| Coverage tensor and self-test | Structure and command passed; 4 maturity, 16 evidence-grade and 11 P0 gaps retained; status `blocked` |
| Documentation inventory and book | Passed; 26 HTML files; development/shipping 3.4.5 |
| Project organization audit and self-test | Passed; source <=800, docs <=2000; tracked debt 0 |

The debug invocations check affected subsets, not extra model coverage. The
full optimized library run includes the added real-candidate replay and all
prior grid/roundoff unit controls. Integration suites and explicit cost tests
are not claimed as freshly executed in this turn.
