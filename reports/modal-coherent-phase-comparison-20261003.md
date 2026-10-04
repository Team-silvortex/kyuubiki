# Modal Coherent Rounding Phase Comparison

Date: 2026-10-03
Source line: daji 3.4.4, local macOS ARM64, Rust 1.88.

## Scope And Decision

This is test-only coherent rounding diagnosis, not new runtime admission or general modal qualification.

Nonlocal one/two-axis phase proposals cross independent closed-entry barriers,
but do not resolve the tiny-coordinate long-beam residual failure. No runtime
algorithm, public API, tolerance, format, dependency or version is changed.
All new search code and the retained-wide normalization helper are test-only.
Expected failed candidates are explicitly asserted, not published or counted
as physical qualifications.

## Proposal Family And Actual Certification

The experiment retains double-double unit-shape components before rounding
to f64. For each nonexact component it finds the adjacent lower/upper f64
bracket and the wide fractional position within that bracket. Exact components
are not moved. Events are sorted by both fraction parts, with equal fractions
flipped simultaneously; rounding the fraction to f64 first would lose some
event order. The prior f64 unit-shape helper delegates to the retained-parts
helper and its rounded output remains covered by the existing regressions.

One-axis proposals sweep a common threshold from all floors through sorted
event groups to all ceilings, at most `n+1` patterns. Two-axis proposals use
independent thresholds on a supplied sorted proper coordinate subset and its
complement. Their Cartesian product has at most `(floor(n/2)+1) * (ceil(n/2)+1)`
patterns. The beam fixture explicitly partitions translations and rotations;
this is supplied model knowledge, not a generic label-free policy.

Cached wide columns of `D * (K - lambda M)`, with `D = diag(1/sqrt(mass))`,
predict each candidate residual by applying coordinate deltas to the actual
nearest-candidate residual. The predicted residual square ranks proposals;
it is not an acceptance certificate, interval enclosure or feasibility proof.
Only the best 64 non-nearest proposals receive original-operator certificates.
One counter covers these plus initial and fresh final checks, at most 66.
Actual relative residual must strictly decrease to retain a candidate. An
initially passing shape, or a searched candidate that passes, still requires
a fresh final certificate. Invalid/worse certificates cannot be overridden by
the prediction. The nearest vector is retained as a separate baseline even
when it is not a threshold pattern because of exact-half tie parity.

The second center translates each retained fractional remainder onto the
actual physical-polisher seed; its nearest f64 coordinates are verified
bitwise against that seed. It is an approximate phase hint, not a newly solved
continuous mode. Neither center is an admissible result until independently
certified. There is no exhaustive subset search, wider ULP-radius search or
iterative recentering in this experiment.

## Actual Long Beam Comparison

All fixtures use 128 elements, 256 active bending coordinates and the actual
rounded physical stiffness/mass. Each scale uses the same test-only wide
first eigenvalue for both centers and all phase/block comparisons. This root
can differ by one ULP from the production root used in prior neighborhood
records; last digits across those records are not same-root comparisons.

Every phase search below rejects at the unchanged `1e-8` gate. Each uses
exactly 66 actual certificates and reports its best true residual on failure.

| Center | Axes | Segment length 1 | Segment length 1e14 | Segment length 1e-10 |
| --- | --- | --- | --- | --- |
| Wide unit shape | One | 4.1675771204455554e-8 | 4.249978230755157e-8 | 3.923492557907729e-8 |
| Wide unit shape | Two | 4.1639091269561445e-8 | 4.238550863817453e-8 | 3.918061706978739e-8 |
| Translated polisher | One | 2.1758468539611626e-8 | 2.0126781835359737e-8 | 1.74042705488802e-8 |
| Translated polisher | Two | 2.1758468539611626e-8 | 2.0126011118486146e-8 | 1.74042705488802e-8 |

The existing automatic block correction then starts from the best actually
checked phase shape. Its separate maximum is 80 certificates; the preceding
66-call cap does not cover this second operation, wide mode construction or
physical-polisher preparation.

| Center | Axes | Segment length 1 | Segment length 1e14 | Segment length 1e-10 |
| --- | --- | --- | --- | --- |
| Wide unit shape | One | 9.166419587639856e-9 | 9.26601891487166e-9 | 3.923034244509162e-8 |
| Wide unit shape | Two | 9.166415478576357e-9 | 9.26601891487166e-9 | 3.9095671571986466e-8 |
| Translated polisher | One | 9.205252988043893e-9 | 7.362945985571824e-9 | 1.74042705488802e-8 |
| Translated polisher | Two | 9.205252988043893e-9 | 7.362945985571824e-9 | 1.74042705488802e-8 |

The eight unit/large-coordinate block shapes pass original-operator checks,
unit-shape norm checks, constructed result JSON bit-preserving readback and
independently reassembled physical residuals. All four tiny-coordinate block
comparisons explicitly reject. These are local candidate records, not public
solver or live Agent outputs. Residual bands are asserted for every phase
failure and every tiny-coordinate block failure; no error-only success test
can disguise an unexpected failure category.

These negative results narrow the two sampled phase families. They do not
establish an f64 residual floor or prove that no admissible vector exists.

## Independent Controls And Recovery

A nine-coordinate binary-entry system has an independent anchor row, seven
chained difference rows and one weak collective row. The differences constrain
the eight coupled coordinates, while the collective residual starts at minus a quarter
ULP. Moving all eight coupled coordinates upward one ULP gives exact zero.
Enumeration of the eight coupled coordinates' neighboring changes involving
at most four moves confirms none improves the initial closed residual square;
moving the independent anchor can only add a nonnegative square. The existing
four-coordinate neighborhood search rejects with relative residual one;
coherent one/two-axis searches recover the exact rounded target across three
sampled coordinate permutations and both seed signs. The supplied fractional
center is deliberately an approximate hint, not the true continuous solution.

A separate four-coordinate closed-entry target cannot be expressed by any
one-axis pattern but is recovered with independent even/odd thresholds. All
ranked scores in this small comparison match independently calculated closed
residual squares. Wide-tail tests distinguish fractions with equal rounded
f64 values, and verify that exact components and fraction ties behave as
specified. These are analytic proposal checks, not general basis invariance
or physical modal qualification.

Both search modes have monotonic 66-call budget traces, malformed initial
and final certificate rejection, passing-search/failing-final rejection,
actual operator failure propagation, and a zero-predicted-score candidate
rejected by its worse actual certificate. Borrowed centers remain bitwise
unchanged. Preparation and search cancellation cover their available safe
points, including final validation; fresh replay runs a known closed-entry
search successfully after each cancellation. Synthetic callbacks in control
tests are not physical acceptance evidence.

Dimensions remain 2 through 256. Nonzero input heads in centers, directions
and actual certificates must stay within `1e-50..=1e50`, with finite bounded
high/low parts. Wide fractions must resolve strictly inside their brackets;
private predicted residuals use the existing wider component guard. These
conservative limits bound the experiment, not all valid modal requests.

At 256 coordinates the separate structural model allows 4,718,592 bytes of
numeric/container payload and 38,797,312 component visits, below 8 MiB and
40,000,000. It allows 257 one-axis patterns or 16,641 two-axis combinations,
but retains only 64 two-axis candidates for true certification. The model
excludes caller models, shared physical-direction construction, wide proposal
factorization/iterations and actual certificate work. It is not elapsed time,
flop count, allocator bookkeeping or measured peak RSS.

## Verification

Eight retained `coherent_phase_` tests cover the controls and twelve actual
beam comparisons. The final eight-test filter passes after strengthening the
exact-half parity, false-prediction and successful-search/failing-final checks.
All four existing `scaled_wide_` tests pass, covering the retained-parts helper's
rounded output and earlier range/cancellation boundaries.

Full Solver regression passes 1,445 tests with zero failures and nine
pre-existing ignored tests across 186 result groups; the eight new tests are
included in that total. Headless modal integration passes all 37 tests.
Strict all-target Solver/CLI Clippy passes without warnings, and workspace
formatting checks pass. Native tensor and operator-validation checks and their
self-tests pass; documentation book/inventory and organization audits also
pass. Operator-validation verifies 59 registry profiles without executing
their commands. The tensor retains four maturity gaps, 16 evidence-grade gaps
and 11 P0 gaps, with daji qualification still blocked. This scoped diagnosis
does not promote those broader gaps or the unresolved tiny-coordinate mode.

Commands from `workers/rust`:

```text
cargo test -p kyuubiki-solver --lib coherent_phase_ --locked --offline -- --nocapture
cargo test -p kyuubiki-solver --lib scaled_wide_ --locked --offline
cargo test -p kyuubiki-solver --locked --offline
cargo test -p kyuubiki-cli --test modal_spectrum_operator --locked --offline
cargo clippy -p kyuubiki-solver -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
cargo fmt --all -- --check
```

No live Agent, installed/remote qualification, packaging, version change or
Git submission is claimed. Source/document limits remain 800/2,000 lines.
